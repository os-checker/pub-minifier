#![feature(rustc_private)]
//! Binary entrypoint that runs rustc analysis and prints collected module usage data as JSON.

extern crate itertools;
extern crate rustc_data_structures;
extern crate rustc_driver;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_public;
extern crate rustc_span;

use rustc_middle::ty::TyCtxt;
use std::ops::ControlFlow;

mod collector;
mod out;
mod reachability;

/// Starts a rustc session and invokes the analysis callback with `TyCtxt`.
fn main() {
    let args: Vec<_> = std::env::args().collect();
    rustc_public::run_with_tcx!(&args, analysis).unwrap();
}

/// Collects module reachability data from HIR and writes the final JSON output.
fn analysis(tcx: TyCtxt) -> ControlFlow<()> {
    let modules = collector::Modules::collect(tcx);
    out::out(&modules.out(tcx));
    ControlFlow::Continue(())
}
