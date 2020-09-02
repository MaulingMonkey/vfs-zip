// NOTE:  This file is excluded from Cargo.toml !
// docs.rs instead configures this via [package.metadata.docs.rs]

use std::process::Command;

fn main() {
    if is_nightly().unwrap_or(false) {
        println!("cargo:rustc-cfg=external_doc");
    }
}

fn is_nightly() -> Result<bool, Box<dyn std::error::Error>> {
    let o = Command::new("rustc").arg("--version").output()?;
    let stdout = String::from_utf8(o.stdout)?;
    let mut fragments = stdout.split_ascii_whitespace();

    let _rustc  = fragments.next().unwrap_or("");                           // "rustc"
    let _ver    = fragments.next().unwrap_or("");                           // "1.47.0-nightly"
    let _hash   = fragments.next().unwrap_or("").trim_start_matches('(');   // "(bf4342114"
    let _date   = fragments.next().unwrap_or("").trim_end_matches(')');     // "2020-08-25)"

    Ok(_ver.ends_with("-nightly"))
}
