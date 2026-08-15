use itertools::Itertools;
use rustc_data_structures::fx::FxHashMap;
use rustc_hir::{
    ItemId, ItemKind, OwnerId,
    def_id::{CRATE_MOD_ID, LocalModId},
};
use rustc_middle::ty::TyCtxt;

use crate::out::{OutModule, def_path_str};

#[derive(Default, Debug)]
pub struct Modules {
    map: FxHashMap<LocalModId, Module>,
}

impl Modules {
    pub fn add_item_id(&mut self, tcx: TyCtxt, item_id: ItemId) {
        let item_kind = tcx.hir_item(item_id).kind;
        if let ItemKind::Mod(_, _) = item_kind {
            let current = owner_id_to_local_mod_id(item_id.owner_id);
            // Deepest first, and iterate to root.
            let v_parent: Vec<_> = tcx
                .hir_parent_owner_iter(item_id.hir_id())
                .map(|parent| owner_id_to_local_mod_id(parent.0))
                .collect();

            // Add current module.
            let direct_parent = v_parent.first().copied().unwrap_or(current);
            self.add_module(current, (v_parent.len() + 1) as u8, direct_parent);

            // Add parent modules: shallow to deep, starting from the next level of root.
            for (idx, &[current, parent]) in v_parent.array_windows().rev().enumerate() {
                self.add_module(current, (idx + 2) as u8, parent);
            }
        }
    }

    pub fn add_root(&mut self, tcx: TyCtxt) {
        let root = CRATE_MOD_ID;
        assert_eq!(
            tcx.hir_get_module(root).0.item_ids,
            tcx.hir_root_module().item_ids,
            "{root:?} is not a root module"
        );
        self.add_module(root, 1, root);
    }

    fn add_module(&mut self, current: LocalModId, level: u8, parent: LocalModId) {
        self.map
            .entry(current)
            .or_insert_with(|| Module { level, parent });
    }

    pub fn out(&self, tcx: TyCtxt) -> Vec<OutModule> {
        self.map
            .iter()
            .map(|(key, val)| OutModule {
                level: val.level,
                name: def_path_str(key.to_def_id(), tcx),
                parent: def_path_str(val.parent.to_def_id(), tcx),
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
}

fn owner_id_to_local_mod_id(owner_id: OwnerId) -> LocalModId {
    LocalModId::new_unchecked(owner_id.def_id)
}
