//! Aggregates per-module item usage and converts collected data into stable output structures.

use itertools::Itertools;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{
    ItemKind, Mod,
    def_id::{CRATE_MOD_ID, DefId, LocalModId},
};
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;

use crate::{
    out::{OutItemUsage, OutModule, OutUsage, def_path_str, span_to_string},
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
            let item_def_id = item.owner_id.to_def_id();
            self.record_usage(current, UsageHit::new_definition(item_def_id, item.span));

            for hit in reachability::collect_item_usages(tcx, item) {
                self.record_usage(current, hit);
            }

            if let ItemKind::Mod(_, _) = item.kind {
                let child = LocalModId::new_unchecked(item.owner_id.def_id);
                self.collect_module(tcx, child, level.saturating_add(1), current);
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
