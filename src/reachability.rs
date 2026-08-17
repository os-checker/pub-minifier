//! Walks HIR nodes inside an item and emits semantic usage hits with spans.

use rustc_hir::{
    AmbigArg, Expr, ExprKind, HirId, Item, ItemKind, Path, QPath, Ty, TyKind, UsePath,
    def::{DefKind, Res},
    def_id::DefId,
    intravisit::{self, Visitor},
};
use rustc_middle::{hir::nested_filter::All, ty::TyCtxt};
use rustc_span::Span;

/// Collects all non-definition usage hits found in a single HIR item.
pub fn collect_item_usages<'tcx>(tcx: TyCtxt<'tcx>, item: &'tcx Item<'tcx>) -> Vec<UsageHit> {
    let mut collector = ReachabilityCollector {
        tcx,
        hits: Vec::new(),
    };
    collector.collect_use_reachability(item);
    collector.visit_item(item);
    collector.hits
}

/// How the item is reached in the module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reachability {
    /// The item is defined in the module.
    Definition,
    /// The function is called in the module.
    Call,
    /// The item is used in type annotation.
    TypeAnnotation,
    /// The item is used as a construct.
    Construct,
    /// The item is imported to the module via `use`.
    Import,
    /// The item is exported from the module via `pub use`.
    Export,
}

#[derive(Clone, Copy)]
pub struct UsageHit {
    pub item: DefId,
    pub reachability: Reachability,
    pub span: Span,
}

impl UsageHit {
    pub fn new_definition(item: DefId, span: Span) -> Self {
        UsageHit {
            item,
            reachability: Reachability::Definition,
            span,
        }
    }
}

struct ReachabilityCollector<'tcx> {
    tcx: TyCtxt<'tcx>,
    hits: Vec<UsageHit>,
}

impl<'tcx> ReachabilityCollector<'tcx> {
    /// Pushes one usage hit into the in-memory hit buffer.
    fn record_usage(&mut self, item: DefId, reachability: Reachability, span: Span) {
        self.hits.push(UsageHit {
            item,
            reachability,
            span,
        });
    }

    /// Handles `use` items and classifies them as import or export.
    fn collect_use_reachability(&mut self, item: &Item<'_>) {
        let ItemKind::Use(path, _) = item.kind else {
            return;
        };
        let reachability = if self.tcx.local_visibility(item.owner_id.def_id).is_public() {
            Reachability::Export
        } else {
            Reachability::Import
        };
        self.record_use_path_res(path, reachability, item.span);
    }

    /// Records all resolved definitions carried by a `use` path.
    fn record_use_path_res(&mut self, path: &UsePath<'_>, reachability: Reachability, span: Span) {
        for res in path.res.present_items() {
            if let Some(def_id) = res.opt_def_id() {
                self.record_usage(def_id, reachability, span);
            }
        }
    }

    /// Records a definition resolved from a normal path.
    fn record_path_res(&mut self, path: &Path<'_>, reachability: Reachability, span: Span) {
        if let Some(def_id) = path.res.opt_def_id() {
            self.record_usage(def_id, reachability, span);
        }
    }

    /// Records a definition resolved from a qualified path form.
    fn record_qpath(&mut self, qpath: &QPath<'_>, reachability: Reachability, span: Span) {
        match qpath {
            QPath::Resolved(_, path) => self.record_path_res(path, reachability, span),
            QPath::TypeRelative(_, segment) => {
                if let Res::Def(_, def_id) = segment.res {
                    self.record_usage(def_id, reachability, span);
                }
            }
        }
    }
}

impl<'tcx> Visitor<'tcx> for ReachabilityCollector<'tcx> {
    type NestedFilter = All;

    /// Provides `TyCtxt` for nested body traversal in intravisit.
    fn maybe_tcx(&mut self) -> Self::MaybeTyCtxt {
        self.tcx
    }

    /// Marks type-path occurrences as `TypeAnnotation` reachability.
    fn visit_ty(&mut self, ty: &'tcx Ty<'tcx, AmbigArg>) {
        if let TyKind::Path(qpath) = ty.kind {
            self.record_qpath(&qpath, Reachability::TypeAnnotation, ty.span);
        }
        intravisit::walk_ty(self, ty);
    }

    /// Marks call and construction expressions with their corresponding reachability.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        match expr.kind {
            ExprKind::Call(callee, ..) => {
                if let ExprKind::Path(qpath) = callee.kind {
                    self.record_qpath(&qpath, Reachability::Call, callee.span);
                }
            }
            ExprKind::MethodCall(_, _, _, _) => {
                if let Some(def_id) = self
                    .tcx
                    .typeck(expr.hir_id.owner.def_id)
                    .type_dependent_def_id(expr.hir_id)
                {
                    self.record_usage(def_id, Reachability::Call, expr.span);
                }
            }
            ExprKind::Struct(qpath, _, _) => {
                self.record_qpath(qpath, Reachability::Construct, expr.span);
            }
            _ => {}
        }

        intravisit::walk_expr(self, expr);
    }

    /// Captures constructor paths that appear outside `ExprKind::Struct`.
    fn visit_path(&mut self, path: &Path<'tcx>, _id: HirId) {
        if matches!(path.res, Res::Def(DefKind::Ctor(..), _))
            && let Some(def_id) = path.res.opt_def_id()
        {
            self.record_usage(def_id, Reachability::Construct, path.span);
        }
        intravisit::walk_path(self, path);
    }

    fn visit_impl_item(&mut self, ii: &'tcx rustc_hir::ImplItem<'tcx>) -> Self::Result {
        let def_id = ii.owner_id.to_def_id();
        self.record_usage(def_id, Reachability::Definition, ii.span);
        intravisit::walk_impl_item(self, ii)
    }

    fn visit_mod(&mut self, m: &'tcx rustc_hir::Mod<'tcx>, _s: Span, _n: HirId) -> Self::Result {
        for &item_id in m.item_ids {
            let item = self.maybe_tcx().hir_item(item_id);
            // Don't walk into modules inside a module.
            if !matches!(item.kind, ItemKind::Mod(..)) {
                self.visit_item(item);
            }
        }
    }
}
