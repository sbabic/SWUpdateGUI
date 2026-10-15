// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Fully initialize LVGL library to show the GUI using /dev/fb and to take
//                 inputs using evdev.

use crate::config::RecoveryParameters;
use crate::lv::{self, sys, Obj};
use crate::system;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

const FRAMEBUFFER_DEVICE: &str = "/dev/fb";
const TOUCHSCREEN_DEVICE: &str = "/dev/input/touchscreen0";

/* Sleep in the endless loop, which process LVGL events */
const LVGL_LOOP_SLEEP: Duration = Duration::from_millis(5);

/* Mouse cursor icon, 14x20 pixels in ARGB8888 */
static MOUSE_CURSOR_ICON: &[u8] = include_bytes!("../assets/mouse_cursor_argb8888.bin");
const MOUSE_CURSOR_WIDTH: u32 = 14;
const MOUSE_CURSOR_HEIGHT: u32 = 20;

extern "C" fn tick_ms() -> u32 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis() as u32
}

fn mouse_cursor_image() -> &'static sys::lv_image_dsc_t {
    // SAFETY: lv_image_dsc_t is a plain C struct; all-zero is a valid initial value.
    let mut dsc: sys::lv_image_dsc_t = unsafe { std::mem::zeroed() };

    dsc.header.set_magic(sys::LV_IMAGE_HEADER_MAGIC);
    dsc.header.set_cf(sys::LV_COLOR_FORMAT_ARGB8888);
    dsc.header.set_w(MOUSE_CURSOR_WIDTH);
    dsc.header.set_h(MOUSE_CURSOR_HEIGHT);
    dsc.data_size = MOUSE_CURSOR_ICON.len() as u32;
    dsc.data = MOUSE_CURSOR_ICON.as_ptr();

    Box::leak(Box::new(dsc))
}

fn initialize_display(params: &RecoveryParameters) {
    let display = unsafe { sys::lv_linux_fbdev_create() };
    if display.is_null() {
        log::error!("lv_linux_fbdev_create() failed.");
        std::process::exit(1);
    }

    let device = lv::cstring(FRAMEBUFFER_DEVICE);
    unsafe {
        sys::lv_linux_fbdev_set_file(display, device.as_ptr());
        sys::lv_display_set_default(display);
    }

    let angle = params.env.screen_orientation_angle;
    let Some(rotation) = system::rotation_from_angle(angle) else {
        log::error!("Invalid screen rotation angle: {angle}.");
        log::error!("Please provide correct value in SCREEN_ORIENTATION_ANGLE environment variable.");
        log::error!("Valid values are: 0, 90, 180, 270.");
        std::process::exit(-1);
    };
    unsafe { sys::lv_display_set_rotation(display, rotation) };

    log::info!("Display initialisation done.");
}

fn initialize_keypad(device: &str) {
    let path = lv::cstring(device);
    let keypad = unsafe { sys::lv_evdev_create(sys::LV_INDEV_TYPE_KEYPAD, path.as_ptr()) };

    if keypad.is_null() {
        log::error!("lv_evdev_create() failed for keypad. The device used: {device}");
    } else {
        unsafe { sys::lv_indev_set_group(keypad, lv::group_default()) };
        log::info!("Keyboard initialized successfully using [{device}].");
    }
}

fn initialize_touchscreen() {
    let path = lv::cstring(TOUCHSCREEN_DEVICE);
    let pointer = unsafe { sys::lv_evdev_create(sys::LV_INDEV_TYPE_POINTER, path.as_ptr()) };

    if pointer.is_null() {
        log::error!("lv_evdev_create() failed for the touchscreen at [{TOUCHSCREEN_DEVICE}]");
        return;
    }

    let cursor = Obj::new_image(Obj::active_screen());
    cursor.image_set_src(mouse_cursor_image());
    unsafe { sys::lv_indev_set_cursor(pointer, cursor.raw()) };

    log::info!("Touchscreen initialized successfully using [{TOUCHSCREEN_DEVICE}]");
}

/// Fully initialize LVGL library to show the image and handle keyboard and mouse or touch screen.
pub fn initialize(params: &RecoveryParameters) {
    unsafe {
        sys::lv_init();
        sys::lv_tick_set_cb(Some(tick_ms));
        sys::lv_group_set_default(sys::lv_group_create());
    }

    initialize_display(params);
    initialize_touchscreen();
    initialize_keypad(&params.config.keypad_dev);
}

/// Process LVGL events in a loop, calling `on_iteration` after each LVGL timer run.
pub fn run(mut on_iteration: impl FnMut()) -> ! {
    loop {
        unsafe { sys::lv_timer_handler() };

        on_iteration();

        std::thread::sleep(LVGL_LOOP_SLEEP);
    }
}
