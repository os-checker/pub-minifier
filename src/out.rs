extern crate serde;
extern crate serde_json;

use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;
use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, Mutex};

pub fn out<T: Serialize>(val: &T) {
    serde_json::to_writer_pretty(std::io::stdout(), val).unwrap();
}

pub fn def_path_str(def_id: DefId, tcx: TyCtxt) -> String {
    use rustc_middle::ty::print::{with_no_trimmed_paths, with_resolve_crate_name};

    static RECORDED: LazyLock<Mutex<FxHashMap<DefId, String>>> = LazyLock::new(Default::default);

    let mut recorded = RECORDED.lock().unwrap();
    recorded
        .entry(def_id)
        .or_insert_with(|| {
            with_no_trimmed_paths!(with_resolve_crate_name!(tcx.def_path_str(def_id)))
        })
        .clone()
}

#[derive(Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutModule {
    pub level: u8,
    pub name: String,
    pub parent: String,
}
