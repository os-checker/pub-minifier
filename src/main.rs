#![feature(rustc_private)]

extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_public;

use rustc_middle::{hir::ModuleItems, ty::TyCtxt};
use std::ops::ControlFlow;

mod module_levels;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    rustc_public::run_with_tcx!(&args, analysis).unwrap();
}

fn analysis(tcx: TyCtxt) -> ControlFlow<()> {
    let items = tcx.hir_crate_items(());
    free_items(items, tcx);
    // dbg!(items);
    ControlFlow::Continue(())
}

fn free_items(items: &ModuleItems, tcx: TyCtxt) {
    let mut modules = module_levels::Modules::default();
    modules.add_root(tcx);
    for item_id in items.free_items() {
        modules.add_item_id(tcx, item_id);
    }
    dbg!(&modules);
}

mod a {
    mod b {}
}
