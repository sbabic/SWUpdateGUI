// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Thin, safe-ish wrapper around the generated LVGL bindings.
//                 LVGL is single threaded: all functions here must only be
//                 called from the GUI thread.

use std::cell::UnsafeCell;
use std::ffi::{c_void, CStr, CString};
use std::ptr::{self, NonNull};

pub use crate::ffi::lvgl as sys;

/* Values of anonymous enums which are not emitted by bindgen */
pub const OPA_TRANSP: sys::lv_opa_t = 0;
pub const OPA_COVER: sys::lv_opa_t = 255;

/* The names of these constants changed between LVGL 9 minor versions, the values did not */
pub const LABEL_LONG_DOT: sys::lv_label_long_mode_t = 1;
pub const LABEL_LONG_SCROLL_CIRCULAR: sys::lv_label_long_mode_t = 3;

/* LVGL 10 renamed LV_DISPLAY_ROTATION_* to LV_ROTATION_* */
#[cfg(not(lvgl_rotation))]
pub use sys::{
    lv_display_rotation_t as Rotation, LV_DISPLAY_ROTATION_0 as ROTATION_0,
    LV_DISPLAY_ROTATION_180 as ROTATION_180, LV_DISPLAY_ROTATION_270 as ROTATION_270,
    LV_DISPLAY_ROTATION_90 as ROTATION_90,
};
#[cfg(lvgl_rotation)]
pub use sys::{
    lv_rotation_t as Rotation, LV_ROTATION_0 as ROTATION_0, LV_ROTATION_180 as ROTATION_180,
    LV_ROTATION_270 as ROTATION_270, LV_ROTATION_90 as ROTATION_90,
};

pub type EventCb = unsafe extern "C" fn(*mut sys::lv_event_t);

/// Convert a Rust string into a C string, dropping interior NUL bytes.
pub fn cstring(text: &str) -> CString {
    CString::new(text.replace('\0', "")).unwrap_or_default()
}

/// Percentage size, the equivalent of `LV_PCT()`.
pub fn pct(value: i32) -> i32 {
    unsafe { sys::lv_pct(value) }
}

pub fn hor_res() -> i32 {
    unsafe { sys::lv_display_get_horizontal_resolution(ptr::null()) }
}

pub fn ver_res() -> i32 {
    unsafe { sys::lv_display_get_vertical_resolution(ptr::null()) }
}

pub fn color_white() -> sys::lv_color_t {
    unsafe { sys::lv_color_white() }
}

pub fn color_black() -> sys::lv_color_t {
    unsafe { sys::lv_color_black() }
}

pub fn palette_main(palette: sys::lv_palette_t) -> sys::lv_color_t {
    unsafe { sys::lv_palette_main(palette) }
}

pub fn palette_lighten(palette: sys::lv_palette_t, level: u8) -> sys::lv_color_t {
    unsafe { sys::lv_palette_lighten(palette, level) }
}

pub fn palette_darken(palette: sys::lv_palette_t, level: u8) -> sys::lv_color_t {
    unsafe { sys::lv_palette_darken(palette, level) }
}

pub fn font_montserrat_14() -> *const sys::lv_font_t {
    ptr::addr_of!(sys::lv_font_montserrat_14)
}

/// Handle to an LVGL widget. Copying the handle does not copy the widget.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Obj(NonNull<sys::lv_obj_t>);

impl Obj {
    fn wrap(raw: *mut sys::lv_obj_t) -> Self {
        Obj(NonNull::new(raw).expect("LVGL failed to create an object"))
    }

    pub fn from_raw(raw: *mut c_void) -> Option<Self> {
        NonNull::new(raw as *mut sys::lv_obj_t).map(Obj)
    }

    pub fn raw(self) -> *mut sys::lv_obj_t {
        self.0.as_ptr()
    }

    /* creation */

    pub fn new_screen() -> Self {
        Self::wrap(unsafe { sys::lv_obj_create(ptr::null_mut()) })
    }

    pub fn active_screen() -> Self {
        Self::wrap(unsafe { sys::lv_screen_active() })
    }

    pub fn new(parent: Obj) -> Self {
        Self::wrap(unsafe { sys::lv_obj_create(parent.raw()) })
    }

    pub fn new_button(parent: Obj) -> Self {
        Self::wrap(unsafe { sys::lv_button_create(parent.raw()) })
    }

    pub fn new_label(parent: Obj) -> Self {
        Self::wrap(unsafe { sys::lv_label_create(parent.raw()) })
    }

    pub fn new_checkbox(parent: Obj) -> Self {
        Self::wrap(unsafe { sys::lv_checkbox_create(parent.raw()) })
    }

    pub fn new_slider(parent: Obj) -> Self {
        Self::wrap(unsafe { sys::lv_slider_create(parent.raw()) })
    }

    pub fn new_textarea(parent: Obj) -> Self {
        Self::wrap(unsafe { sys::lv_textarea_create(parent.raw()) })
    }

    pub fn new_list(parent: Obj) -> Self {
        Self::wrap(unsafe { sys::lv_list_create(parent.raw()) })
    }

    pub fn new_keyboard(parent: Obj) -> Self {
        Self::wrap(unsafe { sys::lv_keyboard_create(parent.raw()) })
    }

    pub fn new_image(parent: Obj) -> Self {
        Self::wrap(unsafe { sys::lv_image_create(parent.raw()) })
    }

    /// Delete the widget and all its children. The handle must not be used afterwards.
    pub fn delete(self) {
        unsafe { sys::lv_obj_delete(self.raw()) }
    }

    /* geometry and layout */

    pub fn set_size(self, width: i32, height: i32) {
        unsafe { sys::lv_obj_set_size(self.raw(), width, height) }
    }

    pub fn set_width(self, width: i32) {
        unsafe { sys::lv_obj_set_width(self.raw(), width) }
    }

    pub fn set_height(self, height: i32) {
        unsafe { sys::lv_obj_set_height(self.raw(), height) }
    }

    pub fn set_align(self, align: sys::lv_align_t) {
        unsafe { sys::lv_obj_set_align(self.raw(), align) }
    }

    pub fn align(self, align: sys::lv_align_t, x: i32, y: i32) {
        unsafe { sys::lv_obj_align(self.raw(), align, x, y) }
    }

    pub fn align_to(self, base: Obj, align: sys::lv_align_t, x: i32, y: i32) {
        unsafe { sys::lv_obj_align_to(self.raw(), base.raw(), align, x, y) }
    }

    pub fn center(self) {
        unsafe { sys::lv_obj_center(self.raw()) }
    }

    pub fn set_flex_flow(self, flow: sys::lv_flex_flow_t) {
        unsafe { sys::lv_obj_set_flex_flow(self.raw(), flow) }
    }

    pub fn set_flex_align(
        self,
        main: sys::lv_flex_align_t,
        cross: sys::lv_flex_align_t,
        track: sys::lv_flex_align_t,
    ) {
        unsafe { sys::lv_obj_set_flex_align(self.raw(), main, cross, track) }
    }

    pub fn set_flex_grow(self, grow: u8) {
        unsafe { sys::lv_obj_set_flex_grow(self.raw(), grow) }
    }

    /* flags and states */

    pub fn add_flag(self, flag: sys::lv_obj_flag_t) {
        unsafe { sys::lv_obj_add_flag(self.raw(), flag) }
    }

    pub fn remove_flag(self, flag: sys::lv_obj_flag_t) {
        unsafe { sys::lv_obj_remove_flag(self.raw(), flag) }
    }

    pub fn set_hidden(self, hidden: bool) {
        if hidden {
            self.add_flag(sys::LV_OBJ_FLAG_HIDDEN);
        } else {
            self.remove_flag(sys::LV_OBJ_FLAG_HIDDEN);
        }
    }

    pub fn disable_scrolling(self) {
        self.remove_flag(sys::LV_OBJ_FLAG_SCROLLABLE);
    }

    pub fn add_state(self, state: sys::lv_state_t) {
        unsafe { sys::lv_obj_add_state(self.raw(), state) }
    }

    pub fn remove_state(self, state: sys::lv_state_t) {
        unsafe { sys::lv_obj_remove_state(self.raw(), state) }
    }

    pub fn has_state(self, state: sys::lv_state_t) -> bool {
        unsafe { sys::lv_obj_has_state(self.raw(), state) }
    }

    /* styles */

    pub fn add_style(self, style: &'static Style, selector: u32) {
        unsafe { sys::lv_obj_add_style(self.raw(), style.raw(), selector) }
    }

    pub fn remove_style(self, style: Option<&'static Style>, selector: u32) {
        let style = style.map_or(ptr::null(), |s| s.raw() as *const _);
        unsafe { sys::lv_obj_remove_style(self.raw(), style, selector) }
    }

    pub fn set_style_pad_all(self, value: i32, selector: u32) {
        unsafe {
            sys::lv_obj_set_style_pad_left(self.raw(), value, selector);
            sys::lv_obj_set_style_pad_right(self.raw(), value, selector);
            sys::lv_obj_set_style_pad_top(self.raw(), value, selector);
            sys::lv_obj_set_style_pad_bottom(self.raw(), value, selector);
        }
    }

    pub fn set_style_pad_row(self, value: i32, selector: u32) {
        unsafe { sys::lv_obj_set_style_pad_row(self.raw(), value, selector) }
    }

    pub fn set_style_bg_color(self, color: sys::lv_color_t, selector: u32) {
        unsafe { sys::lv_obj_set_style_bg_color(self.raw(), color, selector) }
    }

    pub fn set_style_text_align(self, align: sys::lv_text_align_t, selector: u32) {
        unsafe { sys::lv_obj_set_style_text_align(self.raw(), align, selector) }
    }

    /* events */

    /// Register `cb` for events with `code`. `user_data` is returned by [`Event::user_data`].
    pub fn on(self, code: sys::lv_event_code_t, cb: EventCb, user_data: usize) {
        unsafe {
            sys::lv_obj_add_event_cb(self.raw(), Some(cb), code, user_data as *mut c_void);
        }
    }

    /* label */

    pub fn label_set_text(self, text: &str) {
        let text = cstring(text);
        unsafe { sys::lv_label_set_text(self.raw(), text.as_ptr()) }
    }

    pub fn label_set_text_static(self, text: &'static CStr) {
        unsafe { sys::lv_label_set_text_static(self.raw(), text.as_ptr()) }
    }

    pub fn label_set_long_mode(self, mode: sys::lv_label_long_mode_t) {
        unsafe { sys::lv_label_set_long_mode(self.raw(), mode) }
    }

    /* checkbox */

    pub fn checkbox_set_text(self, text: &str) {
        let text = cstring(text);
        unsafe { sys::lv_checkbox_set_text(self.raw(), text.as_ptr()) }
    }

    /* slider */

    pub fn slider_set_value(self, value: i32) {
        unsafe { sys::lv_slider_set_value(self.raw(), value, false) }
    }

    /* text area */

    pub fn textarea_set_text(self, text: &str) {
        let text = cstring(text);
        unsafe { sys::lv_textarea_set_text(self.raw(), text.as_ptr()) }
    }

    pub fn textarea_add_text(self, text: &str) {
        let text = cstring(text);
        unsafe { sys::lv_textarea_add_text(self.raw(), text.as_ptr()) }
    }

    pub fn textarea_text(self) -> String {
        unsafe {
            let text = sys::lv_textarea_get_text(self.raw());
            if text.is_null() {
                String::new()
            } else {
                CStr::from_ptr(text).to_string_lossy().into_owned()
            }
        }
    }

    pub fn textarea_set_one_line(self, enable: bool) {
        unsafe { sys::lv_textarea_set_one_line(self.raw(), enable) }
    }

    pub fn textarea_set_cursor_click_pos(self, enable: bool) {
        unsafe { sys::lv_textarea_set_cursor_click_pos(self.raw(), enable) }
    }

    pub fn textarea_set_placeholder_text(self, text: &str) {
        let text = cstring(text);
        unsafe { sys::lv_textarea_set_placeholder_text(self.raw(), text.as_ptr()) }
    }

    /* keyboard */

    pub fn keyboard_set_textarea(self, textarea: Option<Obj>) {
        let textarea = textarea.map_or(ptr::null_mut(), Obj::raw);
        unsafe { sys::lv_keyboard_set_textarea(self.raw(), textarea) }
    }

    pub fn keyboard_set_mode(self, mode: sys::lv_keyboard_mode_t) {
        unsafe { sys::lv_keyboard_set_mode(self.raw(), mode) }
    }

    /* list */

    pub fn list_add_button(self) -> Obj {
        Self::wrap(unsafe { sys::lv_list_add_button(self.raw(), ptr::null(), c"".as_ptr()) })
    }

    pub fn list_add_text(self, text: &str) -> Obj {
        let text = cstring(text);
        Self::wrap(unsafe { sys::lv_list_add_text(self.raw(), text.as_ptr()) })
    }

    /* image */

    pub fn image_set_src(self, src: &'static sys::lv_image_dsc_t) {
        unsafe { sys::lv_image_set_src(self.raw(), src as *const _ as *const c_void) }
    }

    /* screen */

    pub fn load_screen(self) {
        unsafe { sys::lv_screen_load(self.raw()) }
    }
}

/// Accessors for the event passed to an event callback.
pub struct Event(*mut sys::lv_event_t);

impl Event {
    /// # Safety
    /// `raw` must be the event pointer LVGL passed to the callback.
    pub unsafe fn from_raw(raw: *mut sys::lv_event_t) -> Self {
        Event(raw)
    }

    pub fn target(&self) -> Option<Obj> {
        Obj::from_raw(unsafe { sys::lv_event_get_target(self.0) })
    }

    pub fn current_target(&self) -> Option<Obj> {
        Obj::from_raw(unsafe { sys::lv_event_get_current_target(self.0) })
    }

    pub fn user_data(&self) -> usize {
        unsafe { sys::lv_event_get_user_data(self.0) as usize }
    }
}

/// An LVGL style. Styles are referenced by LVGL objects, so they are leaked and live forever.
pub struct Style(UnsafeCell<sys::lv_style_t>);

impl Style {
    pub fn new() -> &'static Style {
        // SAFETY: lv_style_t is a plain C struct which lv_style_init() initializes.
        let style: &'static Style = Box::leak(Box::new(Style(UnsafeCell::new(unsafe {
            std::mem::zeroed()
        }))));
        unsafe { sys::lv_style_init(style.raw()) };
        style
    }

    fn raw(&self) -> *mut sys::lv_style_t {
        self.0.get()
    }

    pub fn radius(&self, value: i32) -> &Self {
        unsafe { sys::lv_style_set_radius(self.raw(), value) };
        self
    }

    pub fn bg_opa(&self, value: sys::lv_opa_t) -> &Self {
        unsafe { sys::lv_style_set_bg_opa(self.raw(), value) };
        self
    }

    pub fn bg_color(&self, value: sys::lv_color_t) -> &Self {
        unsafe { sys::lv_style_set_bg_color(self.raw(), value) };
        self
    }

    pub fn text_color(&self, value: sys::lv_color_t) -> &Self {
        unsafe { sys::lv_style_set_text_color(self.raw(), value) };
        self
    }

    pub fn text_align(&self, value: sys::lv_text_align_t) -> &Self {
        unsafe { sys::lv_style_set_text_align(self.raw(), value) };
        self
    }

    pub fn text_font(&self, value: *const sys::lv_font_t) -> &Self {
        unsafe { sys::lv_style_set_text_font(self.raw(), value) };
        self
    }

    pub fn border_opa(&self, value: sys::lv_opa_t) -> &Self {
        unsafe { sys::lv_style_set_border_opa(self.raw(), value) };
        self
    }

    pub fn border_width(&self, value: i32) -> &Self {
        unsafe { sys::lv_style_set_border_width(self.raw(), value) };
        self
    }

    pub fn border_color(&self, value: sys::lv_color_t) -> &Self {
        unsafe { sys::lv_style_set_border_color(self.raw(), value) };
        self
    }

    pub fn outline_color(&self, value: sys::lv_color_t) -> &Self {
        unsafe { sys::lv_style_set_outline_color(self.raw(), value) };
        self
    }

    pub fn pad_all(&self, value: i32) -> &Self {
        unsafe {
            sys::lv_style_set_pad_left(self.raw(), value);
            sys::lv_style_set_pad_right(self.raw(), value);
            sys::lv_style_set_pad_top(self.raw(), value);
            sys::lv_style_set_pad_bottom(self.raw(), value);
        }
        self
    }

    pub fn pad_right(&self, value: i32) -> &Self {
        unsafe { sys::lv_style_set_pad_right(self.raw(), value) };
        self
    }

    pub fn translate_y(&self, value: i32) -> &Self {
        unsafe { sys::lv_style_set_translate_y(self.raw(), value) };
        self
    }

    pub fn width(&self, value: i32) -> &Self {
        unsafe { sys::lv_style_set_width(self.raw(), value) };
        self
    }
}

/* input groups and keypad navigation */

pub fn group_default() -> *mut sys::lv_group_t {
    unsafe { sys::lv_group_get_default() }
}

pub fn group_remove_all() {
    unsafe { sys::lv_group_remove_all_objs(group_default()) }
}

pub fn group_add(obj: Obj) {
    unsafe { sys::lv_group_add_obj(group_default(), obj.raw()) }
}

pub fn group_focus(obj: Obj) {
    unsafe { sys::lv_group_focus_obj(obj.raw()) }
}

#[derive(Clone, Copy, Debug)]
pub enum GridNav {
    None,
    Rollover,
    ScrollFirst,
}

/// Enable grid navigation on a container (no-op if LVGL was built without LV_USE_GRIDNAV).
pub fn gridnav_add(obj: Obj, ctrl: GridNav) {
    #[cfg(lvgl_gridnav)]
    unsafe {
        let ctrl = match ctrl {
            GridNav::None => sys::LV_GRIDNAV_CTRL_NONE,
            GridNav::Rollover => sys::LV_GRIDNAV_CTRL_ROLLOVER,
            GridNav::ScrollFirst => sys::LV_GRIDNAV_CTRL_SCROLL_FIRST,
        };
        sys::lv_gridnav_add(obj.raw(), ctrl);
    }
    #[cfg(not(lvgl_gridnav))]
    let _ = (obj, ctrl);
}

/// Add containers to the default group with grid navigation, replacing the previous ones.
pub fn set_navigation(containers: &[(Obj, GridNav)]) {
    group_remove_all();
    for &(obj, ctrl) in containers {
        group_add(obj);
        gridnav_add(obj, ctrl);
    }
}
