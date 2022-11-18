use std::path::PathBuf;

use raen::raen::Build;

#[test]
fn compile() {
    use std::fs;
    fs::remove_dir_all("./target/res").unwrap_or_default();
    fs::remove_dir_all("./target/wit").unwrap_or_default();
    let mut build = Build {
        wasm_opt: true,
        ..Build::default()
    };
    build.cargo.cargo_build.release = true;
    build.cargo.workspace.exclude.push("raen".to_string());
    build.cargo.workspace.workspace = true;
    build.run().unwrap();
    let paths = [
        "rust_counter_tutorial",
        "status_message",
        "status_message_advanced",
        "witgen_dep_dep",
        "witgen_dep",
    ];
    let first = paths.iter().map(|s| format!("target/res/{s}.wasm"));
    let second = paths.iter().map(|s| format!("target/wit/{s}/index.wit"));
    first.chain(second).map(PathBuf::from).for_each(|p| {
        if !p.exists() {
            panic!("{} should exist", p.display())
        }
    });
}
