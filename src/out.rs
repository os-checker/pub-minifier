//! Defines JSON output structures and helper utilities for stable string formatting.

extern crate serde;
extern crate serde_json;

use rustc_data_structures::fx::FxHashMap;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;
use rustc_span::Span;
use serde::{Deserialize, Serialize};
use std::{
    borrow::Cow,
    sync::{LazyLock, Mutex},
};

/// Serializes a value as pretty JSON and writes it to stdout.
pub fn out<T: Serialize>(val: &T) {
    serde_json::to_writer_pretty(std::io::stdout(), val).unwrap();
}

/// Returns a fully qualified, cached definition path string for a `DefId`.
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

/// Converts a span into a human-readable diagnostic location string.
pub fn span_to_string(tcx: TyCtxt, span: Span) -> String {
    let span = span.source_callsite();
    tcx.sess.source_map().span_to_diagnostic_string(span)
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Out {
    pub modules: Vec<OutModule>,
    pub items: Vec<OutLocalAncestor>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutUsage {
    pub reachability: String,
    pub spans: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutItemUsage {
    pub item: String,
    pub kind: Cow<'static, str>,
    pub usages: Vec<OutUsage>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutModule {
    pub level: u8,
    pub name: String,
    pub items: Vec<OutItemUsage>,
    pub parent_mod: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct OutLocalAncestor {
    pub item: String,
    pub kind: Cow<'static, str>,
    pub visibility: String,
    pub restricted_vis: Cow<'static, str>,
    pub shallowest_mod: String,
}
