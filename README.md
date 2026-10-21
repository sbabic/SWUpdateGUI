<p align ="center"><img src=SWUpdate.svg width=400 height=400 /></p>

## SWUpdateGUI - a simple GUI for SWUpdate in rescue mode

The goal is to have a GUI for HMI devices when SWUpdate is started
in rescue mode.

The GUI has the following goal:

- GUI must show when an update is started from one of the interface
  and displays the progress and status.
- It can initiate an install from a local storage as SD / USB
- It can provide stup for SWUpdate like network interface, etc.
- Footprint should be as small as possible to be merged into the rescue image.
- Supported by meta-swupdate

Documentation for this repository is located in [./doc/recovery-gui.md](./doc/recovery-gui.md)

## Building

The recovery GUI is written in Rust and uses [LVGL](https://lvgl.io) 9 or 10 and the
SWUpdate client library (`libswupdate`). Rust bindings for both are generated at
build time with `bindgen`, so `libclang` must be available on the build host.

```sh
cargo build --release
cargo test
```

LVGL is located with pkg-config (`lvgl.pc`), which provides the exact compiler
flags LVGL was built with; this matters, because the generated bindings must see
the same `lv_conf.h` configuration as the library. Without pkg-config, the
headers are searched in the sysroot (`SYSROOT` or `PKG_CONFIG_SYSROOT_DIR`), then
in `/usr/include` and `/usr/local/include`. The search can be adjusted with:

| Variable                   | Meaning                                                      |
|----------------------------|--------------------------------------------------------------|
| `LVGL_INCLUDE_DIR`         | directory containing `lvgl/lvgl.h`; disables pkg-config      |
| `LVGL_LIB_DIR`             | directory containing `liblvgl` (without pkg-config)          |
| `LVGL_LIBS`                | libraries to link, comma separated, replacing pkg-config's (default `lvgl`) |
| `SWUPDATE_INCLUDE_DIR`     | directory containing `progress_ipc.h` and `network_ipc.h`    |
| `SWUPDATE_LIB_DIR`         | directory containing `libswupdate`                           |
| `BINDGEN_EXTRA_CLANG_ARGS` | extra arguments for clang, e.g. `--sysroot=...` when cross-compiling |

Without pkg-config, a static LVGL built with ThorVG also needs the C++ runtime:
`LVGL_LIBS=lvgl,stdc++`.

LVGL must be built with `LV_USE_LINUX_FBDEV`, `LV_USE_EVDEV`, `LV_USE_KEYBOARD` and
`LV_FONT_MONTSERRAT_14`. `LV_USE_GRIDNAV` is needed for navigation with a keypad;
without it the build prints a warning and only touch input is fully supported.

The log level is set with the `RECOVERY_GUI_LOG` environment variable
(`error`, `warn`, `info`, `debug`, `trace`; default `info`).


## License
This project is licensed under GPL version 2.0+.
