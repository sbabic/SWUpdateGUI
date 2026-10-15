// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
// Generates Rust bindings for LVGL (v9) and libswupdate and sets up linking.
//
// The following environment variables can be used to locate the libraries,
// for example when cross-compiling inside a Yocto SDK:
//
//   SYSROOT / PKG_CONFIG_SYSROOT_DIR  sysroot used for default search paths
//   LVGL_INCLUDE_DIR      directory containing `lvgl/lvgl.h` and `lvgl/lv_conf.h`
//   LVGL_LIB_DIR          directory containing liblvgl
//   LVGL_LIBS             comma separated libraries to link (default: "lvgl")
//   SWUPDATE_INCLUDE_DIR  directory containing `progress_ipc.h`, `network_ipc.h`
//   SWUPDATE_LIB_DIR      directory containing libswupdate
//
// Extra clang arguments for bindgen can be passed with BINDGEN_EXTRA_CLANG_ARGS.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn env_path(name: &str) -> Option<PathBuf> {
    println!("cargo:rerun-if-env-changed={name}");
    env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn sysroot() -> Option<PathBuf> {
    env_path("SYSROOT").or_else(|| env_path("PKG_CONFIG_SYSROOT_DIR"))
}

/// Find the first candidate directory containing `probe`.
fn find_dir(candidates: &[PathBuf], probe: &str) -> Option<PathBuf> {
    candidates.iter().find(|d| d.join(probe).exists()).cloned()
}

fn default_include_dirs(sysroot: &Option<PathBuf>) -> Vec<PathBuf> {
    match sysroot {
        Some(root) => vec![root.join("usr/include"), root.join("usr/local/include")],
        None => vec![PathBuf::from("/usr/include"), PathBuf::from("/usr/local/include")],
    }
}

fn link_search(dir: &Path) {
    println!("cargo:rustc-link-search=native={}", dir.display());
}

fn sysroot_clang_args(sysroot: &Option<PathBuf>) -> Vec<String> {
    sysroot
        .iter()
        .map(|root| format!("--sysroot={}", root.display()))
        .collect()
}

fn generate_lvgl(out: &Path, sysroot: &Option<PathBuf>) {
    let inc = env_path("LVGL_INCLUDE_DIR")
        .or_else(|| find_dir(&default_include_dirs(sysroot), "lvgl/lvgl.h"))
        .expect("LVGL headers not found; set LVGL_INCLUDE_DIR");

    if let Some(dir) = env_path("LVGL_LIB_DIR") {
        link_search(&dir);
    } else if inc.ends_with("local/include") {
        link_search(&inc.with_file_name("lib"));
    }

    println!("cargo:rerun-if-env-changed=LVGL_LIBS");
    let libs = env::var("LVGL_LIBS").unwrap_or_else(|_| "lvgl".into());
    for lib in libs.split(',').map(str::trim).filter(|l| !l.is_empty()) {
        println!("cargo:rustc-link-lib={lib}");
    }
    println!("cargo:rustc-link-lib=m");

    let bindings = bindgen::Builder::default()
        .header("wrapper/lvgl.h")
        .clang_arg(format!("-I{}", inc.display()))
        .clang_arg(format!("-I{}", inc.join("lvgl").display()))
        .clang_args(sysroot_clang_args(sysroot))
        .allowlist_function("lv_.*")
        .allowlist_type("lv_.*")
        .allowlist_var("LV_.*")
        .allowlist_var("lv_font_.*")
        .prepend_enum_name(false)
        .layout_tests(false)
        .generate_comments(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("unable to generate LVGL bindings");

    let path = out.join("lvgl_bindings.rs");
    bindings.write_to_file(&path).expect("unable to write LVGL bindings");

    // The grid navigation helper is optional in lv_conf.h. Without it the
    // GUI still works with touch, only keypad navigation is reduced.
    let text = fs::read_to_string(&path).unwrap();
    if text.contains("pub fn lv_gridnav_add(") {
        println!("cargo:rustc-cfg=lvgl_gridnav");
    } else {
        println!("cargo:warning=LVGL was built without LV_USE_GRIDNAV; keypad navigation is limited");
    }
}

fn generate_swupdate(out: &Path, sysroot: &Option<PathBuf>) {
    let inc = env_path("SWUPDATE_INCLUDE_DIR")
        .or_else(|| find_dir(&default_include_dirs(sysroot), "progress_ipc.h"))
        .expect("libswupdate headers not found; set SWUPDATE_INCLUDE_DIR");

    if let Some(dir) = env_path("SWUPDATE_LIB_DIR") {
        link_search(&dir);
    }
    println!("cargo:rustc-link-lib=swupdate");

    let bindings = bindgen::Builder::default()
        .header("wrapper/swupdate.h")
        .clang_arg(format!("-I{}", inc.display()))
        .clang_args(sysroot_clang_args(sysroot))
        .allowlist_function("progress_ipc_connect")
        .allowlist_function("progress_ipc_receive")
        .allowlist_function("ipc_notify_connect")
        .allowlist_function("ipc_notify_receive")
        .allowlist_function("swupdate_prepare_req")
        .allowlist_function("swupdate_async_start")
        .allowlist_type("RECOVERY_STATUS")
        .allowlist_type("sourcetype")
        .allowlist_type("run_type")
        .prepend_enum_name(false)
        .layout_tests(false)
        .generate_comments(false)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate()
        .expect("unable to generate libswupdate bindings");

    bindings
        .write_to_file(out.join("swupdate_bindings.rs"))
        .expect("unable to write libswupdate bindings");
}

fn main() {
    println!("cargo:rustc-check-cfg=cfg(lvgl_gridnav)");
    println!("cargo:rerun-if-changed=wrapper");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let sysroot = sysroot();

    generate_lvgl(&out, &sysroot);
    generate_swupdate(&out, &sysroot);
}
