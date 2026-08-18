//! Aggregates per-module item usage and converts collected data into stable output structures.

use itertools::Itertools;
use rustc_data_structures::fx::{FxHashMap, FxHashSet};
use rustc_hir::{
    ItemKind, Mod,
    def::DefKind,
    def_id::{CRATE_MOD_ID, DefId, LocalModId},
};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

use crate::{
    out::{OutItemUsage, OutLocalAncestor, OutModule, OutUsage, def_path_str, span_to_string},
    reachability::{self, Reachability, UsageHit},
};

#[derive(Default, Debug)]
pub struct Modules {
    map: FxHashMap<LocalModId, Module>,
}

impl Modules {
    /// Builds the full module tree from crate root and collects per-module item usage.
    pub fn collect(tcx: TyCtxt) -> Self {
        let mut modules = Self::default();
        modules.collect_module(tcx, CRATE_MOD_ID, 1, CRATE_MOD_ID);
        modules
    }

    /// Recursively traverses one module, collecting definition and usage hits for its items.
    fn collect_module(&mut self, tcx: TyCtxt, current: LocalModId, level: u8, parent: LocalModId) {
        let module = if current == CRATE_MOD_ID {
            tcx.hir_root_module()
        } else {
            tcx.hir_get_module(current).0
        };

        self.add_module(current, module, level, parent);

        for item_id in module.item_ids.iter().copied() {
            let item = tcx.hir_item(item_id);
            let item_def_id = item_id.owner_id.to_def_id();
            self.record_usage(current, UsageHit::new_definition(item_def_id, item.span));

            if let ItemKind::Mod(_, _) = item.kind {
                let child = LocalModId::new_unchecked(item.owner_id.def_id);
                self.collect_module(tcx, child, level.saturating_add(1), current);
            } else {
                for hit in reachability::collect_item_usages(tcx, item) {
                    self.record_usage(current, hit);
                }
            }
        }
    }

    /// Inserts a module record if it has not been seen before.
    fn add_module(&mut self, current: LocalModId, _module: &Mod, level: u8, parent: LocalModId) {
        self.map.entry(current).or_insert_with(|| Module {
            level,
            parent,
            items: FxHashMap::default(),
        });
    }

    /// Records one usage hit for an item in the current module.
    fn record_usage(&mut self, current_mod: LocalModId, hit: UsageHit) {
        if let Some(module) = self.map.get_mut(&current_mod) {
            module
                .items
                .entry(hit.item)
                .or_default()
                .add(hit.reachability, hit.span);
        } else {
            eprintln!("{current_mod:?} is not recorded in the module map");
        }
    }

    /// Converts collected module usage state into sorted, serializable output.
    pub fn out(&self, tcx: TyCtxt) -> Vec<OutModule> {
        self.map
            .iter()
            .map(|(key, val)| OutModule {
                level: val.level,
                name: def_path_str(key.to_def_id(), tcx),
                items: val
                    .items
                    .iter()
                    .map(|(&def_id, usage)| OutItemUsage {
                        item: def_path_str(def_id, tcx),
                        kind: tcx.def_kind_descr(tcx.def_kind(def_id), def_id).into(),
                        usages: usage
                            .entries()
                            .map(|(reachability, spans)| OutUsage {
                                reachability: format!("{reachability:?}"),
                                spans: spans
                                    .iter()
                                    .copied()
                                    .map(|span| span_to_string(tcx, span))
                                    .sorted()
                                    .collect(),
                            })
                            .sorted()
                            .collect(),
                    })
                    .sorted()
                    .collect(),
                parent_mod: def_path_str(val.parent.to_def_id(), tcx),
            })
            .sorted()
            .collect()
    }

    fn local_item_usage(&self) -> Items<'_> {
        let mut map = Items::with_capacity_and_hasher(512, Default::default());
        for (&local_mod_id, module) in &self.map {
            for (&def_id, usage) in &module.items {
                let mut def_span = None;
                if let Some(v_span) = usage.usage.get(&Reachability::Definition) {
                    assert!(
                        v_span.len() == 1,
                        "The def span of {def_id:?} should be unique, \
                         but got multiple def spans:\n{v_span:?}"
                    );
                    def_span = Some(v_span[0]);
                }
                map.entry(def_id)
                    .and_modify(|def| {
                        if let Some(span) = def_span {
                            assert!(
                                def.def_span.is_none(),
                                "{def_id:?} should be defined only once, \
                                 but there are two def spans: {:?} and {span:?}",
                                def.def_span,
                            );
                            def.def_span = def_span;
                        }
                        let inserted = def.used_in_modules.insert(local_mod_id, usage);
                        assert!(
                            inserted.is_none(),
                            "{local_mod_id:?} shouldn't be inserted twice: {usage:?} and {inserted:?}"
                        );
                    })
                    .or_insert_with(|| LocalItemUsage {
                        def_span,
                        def_in_module: def_span.map(|_| local_mod_id),
                        used_in_modules: FxHashMap::from_iter([(local_mod_id, usage)]),
                    });
            }
        }
        map
    }

    pub fn local_ancestor(&self, tcx: TyCtxt) -> Vec<OutLocalAncestor> {
        let local_items = self.local_item_usage();
        let mut map =
            LocalAncestor::with_capacity_and_hasher(local_items.len(), Default::default());
        for (def_id, item) in local_items {
            if item.def_span.is_none() {
                // Skip items that are not locally defined.
                continue;
            }
            if matches!(
                tcx.def_kind(def_id),
                DefKind::Impl { .. }
                    | DefKind::Ctor(..)
                    | DefKind::ExternCrate
                    | DefKind::ForeignTy
                    | DefKind::ForeignMod
            ) {
                // Skip items that we're not interested at.
                continue;
            }

            let shallowest_id = self.shallowest_mod(item.used_in_modules.into_keys());
            map.insert(def_id, shallowest_id);
        }
        map.into_iter()
            .map(|(item_id, local_mod_id)| OutLocalAncestor {
                item: def_path_str(item_id, tcx),
                kind: tcx.def_kind_descr(tcx.def_kind(item_id), item_id).into(),
                shallowest_mod: def_path_str(local_mod_id.to_def_id(), tcx),
            })
            .sorted_unstable()
            .collect()
    }

    fn shallowest_mod(&self, v_mod: impl IntoIterator<Item = LocalModId>) -> LocalModId {
        let mut buf = FxHashSet::with_capacity_and_hasher(self.map.len(), Default::default());
        v_mod
            .into_iter()
            .reduce(|shallow, m| self.pick_shallow_mod(shallow, m, &mut buf))
            .unwrap()
    }

    fn pick_shallow_mod(
        &self,
        m1: LocalModId,
        m2: LocalModId,
        buf: &mut FxHashSet<LocalModId>,
    ) -> LocalModId {
        buf.clear();
        let mut target = m1;
        loop {
            if !buf.insert(target) {
                break;
            }
            target = self.map.get(&target).unwrap().parent;
        }
        target = m2;
        let root = CRATE_MOD_ID;
        loop {
            if buf.contains(&target) {
                return target;
            }
            target = self.map.get(&target).unwrap().parent;
            if target == root {
                return root;
            }
        }
    }
}

#[derive(Debug)]
pub struct Module {
    // Level should start from 1; suppose level exceeding `u8::MAX` won't happen.
    level: u8,
    // Each module has a parent. Root module has itself as parent.
    parent: LocalModId,
    /// Items used in the module.
    /// The same item can appear in multiple syntax locations as different usage.
    items: FxHashMap<DefId, ItemUsage>,
}

#[derive(Debug, Default)]
pub struct ItemUsage {
    usage: FxHashMap<Reachability, Vec<Span>>,
}

impl ItemUsage {
    /// Appends one usage occurrence under a specific reachability category.
    pub fn add(&mut self, reachability: Reachability, span: Span) {
        self.usage.entry(reachability).or_default().push(span);
    }

    /// Returns all reachability buckets and their recorded spans.
    pub fn entries(&self) -> impl Iterator<Item = (&Reachability, &Vec<Span>)> {
        self.usage.iter()
    }
}

/// DefId can be external, because items can be imported
type Items<'a> = FxHashMap<DefId, LocalItemUsage<'a>>;

struct LocalItemUsage<'a> {
    def_span: Option<Span>,
    def_in_module: Option<LocalModId>,
    used_in_modules: FxHashMap<LocalModId, &'a ItemUsage>,
}

/// Only locally defined items will be here.
/// The LocalModId is the shallowest
type LocalAncestor = FxHashMap<DefId, LocalModId>;
