extern crate compiletest_rs as compiletest;

use std::io::ErrorKind;
use std::path::PathBuf;
use std::{env, fs};

fn run_mode(mode: &'static str) {
    let bin = env::var("CARGO_PKG_NAME").unwrap();
    let bin_path = PathBuf::from(env::var(format!("CARGO_BIN_EXE_{bin}")).unwrap());
    dbg!(&bin_path);

    let target_dir = bin_path.parent().unwrap().parent().unwrap();
    let out_dir = target_dir.join("compiletest");
    match fs::remove_dir_all(&out_dir) {
        Ok(()) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => panic!("Failed to remove dir {out_dir:?}: {err:?}"),
    }
    fs::create_dir(&out_dir).unwrap();

    let config = compiletest::Config {
        mode: mode.parse().expect("Invalid mode"),
        src_base: PathBuf::from(format!("tests/{}", mode)),
        // Update ui snapshots via `BLESS=1`.
        bless: env::var("BLESS").is_ok_and(|v| v != "0"),
        rustc_path: bin_path,
        ..Default::default()
    };

    // config.link_deps(); // Populate config.target_rustcflags with dependencies on the path
    config.clean_rmeta(); // If your tests import the parent crate, this helps with E0464

    compiletest::run_tests(&config);
}

#[test]
fn compile_test() {
    run_mode("ui");
}
