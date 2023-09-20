#[cfg(feature = "bindgen")]
extern crate bindgen;

use anyhow::Result;
use std::env;

fn main() -> Result<()> {
    // If we are in docs.rs, there is no need to actually link.
    if std::env::var("DOCS_RS").is_ok() {
        return Ok(());
    }

    // By default, we will just use a pre-generated bindings.rs file. If this feature is turned
    // on, we'll re-generate the bindings at build time.
    #[cfg(feature = "bindgen")]
    generate_bindings()?;

    link_wireshark()?;
    Ok(())
}

fn link_wireshark() -> Result<()> {
    // pkg-config will handle everything for us
    if pkg_config::probe_library("wireshark").is_ok() {
        return Ok(());
    }

    // Default wireshark libraray installed on windows
    #[cfg(target_os = "windows")]
    println!( "cargo:rustc-link-search=native={}", "C:\\Program Files\\Wireshark");

    // Default wireshark libraray installed on macos
    #[cfg(target_os = "macos")]
    println!( "cargo:rustc-link-search=native={}", "/Applications/Wireshark.app/Contents/Frameworks");

    // Specify the wireshark library directory by the environmental variable
    println!("cargo:rerun-if-env-changed=WIRESHARK_LIB_DIR");
    if let Ok(libws_dir) = env::var("WIRESHARK_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", libws_dir);
    }

    println!("cargo:rustc-link-lib=dylib=wireshark");

    Ok(())
}

#[cfg(feature = "bindgen")]
fn generate_bindings() -> Result<()> {
    let mut builder = bindgen::Builder::default()
        .header("wrapper.h")
        .generate_comments(false);

    match pkg_config::probe_library("wireshark") {
        Ok(libws) => {
            for path in libws.include_paths {
                builder = builder.clang_arg(format!("-I{}", path.to_string_lossy()));
            }
        }
        Err(_) => {
            let glib = pkg_config::Config::new().probe("glib-2.0")?;

            for path in glib.include_paths {
                builder = builder.clang_arg(format!("-I{}", path.to_string_lossy()));
            }

            download_wireshark()?;
            let dst = build_wireshark();

            let mut ws_headers_path = dst;
            ws_headers_path.push("include");
            ws_headers_path.push("wireshark");

            builder = builder.clang_arg(format!("-I{}", ws_headers_path.to_string_lossy()));
        }
    }

    let bindings = builder.generate()?;

    use std::path::PathBuf;
    let out_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    bindings.write_to_file(out_path.join("bindings.rs"))?;

    Ok(())
}
