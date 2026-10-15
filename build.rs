// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
// Generates Rust bindings for LVGL (v9, v10) and libswupdate and sets up linking.
//
// LVGL is located with pkg-config (lvgl.pc), which provides the exact compiler
// flags LVGL was built with. Without pkg-config, or to override it, use the
// following environment variables, for example when cross-compiling inside a
// Yocto SDK:
//
//   SYSROOT / PKG_CONFIG_SYSROOT_DIR  sysroot used for default search paths
//   LVGL_INCLUDE_DIR      directory containing `lvgl/lvgl.h`; disables pkg-config
//   LVGL_LIB_DIR          directory containing liblvgl (without pkg-config)
//   LVGL_LIBS             comma separated libraries to link, replacing the
//                         ones from pkg-config (default: "lvgl")
//   SWUPDATE_INCLUDE_DIR  directory containing `progress_ipc.h`, `network_ipc.h`
//   SWUPDATE_LIB_DIR      directory containing libswupdate
//
// Extra clang arguments for bindgen can be passed with BINDGEN_EXTRA_CLANG_ARGS.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn env_path(name: &str) -> Option<PathBuf> {
    println!("cargo:rerun-if-env-changed={name}");
    env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
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
        None => vec![
            PathBuf::from("/usr/include"),
            PathBuf::from("/usr/local/include"),
        ],
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

fn link_libs(libs: &str) {
    for lib in libs.split(',').map(str::trim).filter(|l| !l.is_empty()) {
        println!("cargo:rustc-link-lib={lib}");
    }
}

/// Link setup and clang arguments for LVGL headers in `inc`, without pkg-config.
fn lvgl_manual(inc: &Path, libs: Option<&str>) -> Vec<String> {
    if let Some(dir) = env_path("LVGL_LIB_DIR") {
        link_search(&dir);
    } else if inc.ends_with("local/include") {
        link_search(&inc.with_file_name("lib"));
    }
    link_libs(libs.unwrap_or("lvgl"));
    println!("cargo:rustc-link-lib=m");

    let mut clang_args = vec![
        format!("-I{}", inc.display()),
        format!("-I{}", inc.join("lvgl").display()),
    ];
    // LVGL >= 9.5 installs lv_conf.h into lvgl/config and expects LV_CONF_INCLUDE_SIMPLE
    let conf_dir = inc.join("lvgl/config");
    if conf_dir.join("lv_conf.h").exists() {
        clang_args.push(format!("-I{}", conf_dir.display()));
        clang_args.push("-DLV_CONF_INCLUDE_SIMPLE".into());
    }
    clang_args
}

/// Link setup and clang arguments for LVGL from pkg-config, if lvgl.pc is found.
fn lvgl_pkg_config(libs: Option<&str>) -> Option<Vec<String>> {
    let lib = match pkg_config::Config::new()
        .statik(true)
        .cargo_metadata(libs.is_none())
        .probe("lvgl")
    {
        Ok(lib) => lib,
        Err(err) => {
            let err = err.to_string().replace('\n', " ");
            println!("cargo:warning=lvgl not found with pkg-config, using default paths: {err}");
            return None;
        }
    };

    if let Some(libs) = libs {
        lib.link_paths.iter().for_each(|dir| link_search(dir));
        link_libs(libs);
    }

    let includes = lib
        .include_paths
        .iter()
        .map(|p| format!("-I{}", p.display()));
    let defines = lib.defines.iter().map(|(name, value)| match value {
        Some(value) => format!("-D{name}={value}"),
        None => format!("-D{name}"),
    });
    Some(includes.chain(defines).collect())
}

fn generate_lvgl(out: &Path, sysroot: &Option<PathBuf>) {
    println!("cargo:rerun-if-env-changed=LVGL_LIBS");
    let libs = env::var("LVGL_LIBS").ok().filter(|l| !l.is_empty());
    let libs = libs.as_deref();

    let clang_args = match env_path("LVGL_INCLUDE_DIR") {
        Some(inc) => lvgl_manual(&inc, libs),
        None => lvgl_pkg_config(libs).unwrap_or_else(|| {
            let inc = find_dir(&default_include_dirs(sysroot), "lvgl/lvgl.h")
                .expect("LVGL headers not found; set LVGL_INCLUDE_DIR");
            lvgl_manual(&inc, libs)
        }),
    };

    let bindings = bindgen::Builder::default()
        .header("wrapper/lvgl.h")
        .clang_args(clang_args)
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
    bindings
        .write_to_file(&path)
        .expect("unable to write LVGL bindings");

    // The grid navigation helper is optional in lv_conf.h. Without it the
    // GUI still works with touch, only keypad navigation is reduced.
    let text = fs::read_to_string(&path).unwrap();
    if text.contains("pub fn lv_gridnav_add(") {
        println!("cargo:rustc-cfg=lvgl_gridnav");
    } else {
        println!(
            "cargo:warning=LVGL was built without LV_USE_GRIDNAV; keypad navigation is limited"
        );
    }

    // LVGL 10 renamed LV_DISPLAY_ROTATION_* to LV_ROTATION_*
    if text.contains("pub const LV_ROTATION_90:") {
        println!("cargo:rustc-cfg=lvgl_rotation");
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
    println!("cargo:rustc-check-cfg=cfg(lvgl_rotation)");
    println!("cargo:rerun-if-changed=wrapper");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let sysroot = sysroot();

    generate_lvgl(&out, &sysroot);
    generate_swupdate(&out, &sysroot);
}
