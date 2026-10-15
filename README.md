Documentation for this repository is located in [./doc/recovery-gui.md](./doc/recovery-gui.md)

## Building

The recovery GUI is written in Rust and uses [LVGL](https://lvgl.io) 9 and the
SWUpdate client library (`libswupdate`). Rust bindings for both are generated at
build time with `bindgen`, so `libclang` must be available on the build host.

```sh
cargo build --release
cargo test
```

The libraries are searched in the sysroot (`SYSROOT` or `PKG_CONFIG_SYSROOT_DIR`),
then in `/usr/include` and `/usr/local/include`. The search can be adjusted with:

| Variable                   | Meaning                                                      |
|----------------------------|--------------------------------------------------------------|
| `LVGL_INCLUDE_DIR`         | directory containing `lvgl/lvgl.h` and `lvgl/lv_conf.h`      |
| `LVGL_LIB_DIR`             | directory containing `liblvgl`                               |
| `LVGL_LIBS`                | libraries to link, comma separated (default `lvgl`)          |
| `SWUPDATE_INCLUDE_DIR`     | directory containing `progress_ipc.h` and `network_ipc.h`    |
| `SWUPDATE_LIB_DIR`         | directory containing `libswupdate`                           |
| `BINDGEN_EXTRA_CLANG_ARGS` | extra arguments for clang, e.g. `--sysroot=...` when cross-compiling |

If LVGL is a static library built with ThorVG, add the C++ runtime:
`LVGL_LIBS=lvgl,stdc++`.

LVGL must be built with `LV_USE_LINUX_FBDEV`, `LV_USE_EVDEV`, `LV_USE_KEYBOARD` and
`LV_FONT_MONTSERRAT_14`. `LV_USE_GRIDNAV` is needed for navigation with a keypad;
without it the build prints a warning and only touch input is fully supported.

The log level is set with the `RECOVERY_GUI_LOG` environment variable
(`error`, `warn`, `info`, `debug`, `trace`; default `info`).
