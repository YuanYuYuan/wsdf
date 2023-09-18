#[cfg(feature = "bindgen")]
extern crate bindgen;

use anyhow::Result;
use flate2::read::GzDecoder;
use std::env;
use std::path::PathBuf;
use tar::Archive;

const WIRESHARK_VERSION: &str = "v4.1.1rc0";
const WIRESHARK_SOURCE_DIR: &str = "wireshark";

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
    if pkg_config::probe_library("wireshark").is_ok() {
        // pkg-config will handle everything for us
        return;
    }

    println!("cargo:rustc-link-lib=dylib=wireshark");

    if let Ok(libws_dir) = env::var("WIRESHARK_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", libws_dir);
    } else {
        download_and_build_wireshark()?;
    }

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
            let glib = pkg_config::Config::new()
                .probe("glib-2.0")?;

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

    let bindings = builder
        .generate()?;

    let out_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))?;

    Ok(())
}

fn download_and_build_wireshark() -> Result<()> {
    // We'll have to pull wireshark in and build it...
    println!("cargo:warning=libwireshark was not found, will be built from source");

    download_wireshark()?;
    let dst = build_wireshark();

    let mut dylib_dir = dst;
    dylib_dir.push("lib");

    println!(
        "cargo:rustc-link-search=native={}",
        dylib_dir.to_string_lossy()
    );
    Ok(())
}

fn download_wireshark() -> Result<()> {
    let file_name = format!("wireshark-{WIRESHARK_VERSION}.tar.gz");
    let url = format!(
        "https://gitlab.com/wireshark/wireshark/-/archive/{WIRESHARK_VERSION}/{file_name}"
    );
    let response = reqwest::blocking::get(url)?;
    let bytes = response.bytes()?.to_vec();
    let readable = GzDecoder::new(bytes.as_slice());
    let mut archive = Archive::new(readable);
    archive.unpack(".")?;
    if std::path::Path::new(WIRESHARK_SOURCE_DIR).exists() {
        std::fs::remove_dir_all(WIRESHARK_SOURCE_DIR)?;
    }
    std::fs::rename(format!("wireshark-{WIRESHARK_VERSION}"), WIRESHARK_SOURCE_DIR)?;
    Ok(())
}

fn build_wireshark() -> PathBuf {
    cmake::Config::new(WIRESHARK_SOURCE_DIR)
        .define("BUILD_androiddump", "OFF")
        .define("BUILD_capinfos", "OFF")
        .define("BUILD_captype", "OFF")
        .define("BUILD_ciscodump", "OFF")
        .define("BUILD_corbaidl2wrs", "OFF")
        .define("BUILD_dcerpcidl2wrs", "OFF")
        .define("BUILD_dftest", "OFF")
        .define("BUILD_dpauxmon", "OFF")
        .define("BUILD_dumpcap", "OFF")
        .define("BUILD_editcap", "OFF")
        .define("BUILD_etwdump", "OFF")
        .define("BUILD_logray", "OFF")
        .define("BUILD_mergecap", "OFF")
        .define("BUILD_randpkt", "OFF")
        .define("BUILD_randpktdump", "OFF")
        .define("BUILD_rawshark", "OFF")
        .define("BUILD_reordercap", "OFF")
        .define("BUILD_sshdump", "OFF")
        .define("BUILD_text2pcap", "OFF")
        .define("BUILD_tfshark", "OFF")
        .define("BUILD_tshark", "OFF")
        .define("BUILD_wifidump", "OFF")
        .define("BUILD_wireshark", "OFF")
        .define("BUILD_xxx2deb", "OFF")
        .build()
}
