#[path = "src/pe.rs"]
mod pe;

use std::fmt::Write;

const ORIGINAL_ENV: &str = "BITURBO_ORIGINAL";
const ORIGINAL_MODULE: &str = "biturbo_original";
const OWN_EXPORTS: &[&str] = &["bt_highlight_syntax", "bt_release_highlight_syntax", "biturbo_spiral_proxy"];

fn main() {
    println!("cargo:rerun-if-env-changed={ORIGINAL_ENV}");
    let Ok(original) = std::env::var(ORIGINAL_ENV) else { return };
    println!("cargo:rerun-if-changed={original}");
    let bytes = std::fs::read(&original).unwrap_or_else(|error| panic!("{ORIGINAL_ENV}={original}: {error}"));
    let names = pe::export_names(&bytes).unwrap_or_else(|error| panic!("{original}: {error}"));
    let mut def = String::from("EXPORTS\n");
    for name in OWN_EXPORTS {
        let _ = writeln!(def, "    {name}");
    }
    for name in names.iter().filter(|name| !OWN_EXPORTS.contains(&name.as_str())) {
        let _ = writeln!(def, "    {name}={ORIGINAL_MODULE}.{name}");
    }
    let path = std::path::Path::new(&std::env::var("OUT_DIR").expect("OUT_DIR")).join("forwarders.def");
    std::fs::write(&path, def).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    println!("cargo:rustc-cdylib-link-arg=/DEF:{}", path.display());
}
