// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains common functions for all UI screens
//
// Every screen keeps its widgets and data in a thread-local state, created once at startup.
// LVGL event callbacks access the state through the `with()` function of the screen module.

use crate::config::Config;
use crate::lv::{self, sys, Obj};
use crate::styles::{self, NotifyType};

pub mod file_browser;
pub mod main_screen;
pub mod network_setup;
pub mod progress;

pub const HEADER_HEIGHT: i32 = 50;

/// Declares the thread-local state of a screen with the `install()` and `with()` accessors.
macro_rules! screen_state {
    ($ty:ty) => {
        thread_local! {
            static STATE: std::cell::RefCell<Option<$ty>> = const { std::cell::RefCell::new(None) };
        }

        fn install(screen: $ty) {
            STATE.with(|cell| *cell.borrow_mut() = Some(screen));
        }

        /// Run `f` with the screen state. Returns None if the screen was not created yet,
        /// or if it is already in use further up the call stack.
        fn with<R>(f: impl FnOnce(&mut $ty) -> R) -> Option<R> {
            STATE.with(|cell| match cell.try_borrow_mut() {
                Ok(mut state) => state.as_mut().map(f),
                Err(_) => {
                    log::error!("Re-entrant access to {} ignored", stringify!($ty));
                    None
                }
            })
        }
    };
}
pub(crate) use screen_state;

/// Create all screens. Must be called once, after LVGL was initialized.
pub fn create_all(config: &Config) {
    main_screen::create(config);
    file_browser::create(config);
    progress::create(config);
    network_setup::create(config);
}

/// Parts every screen has: a header at the top and a notification area at the bottom.
pub struct Screen {
    /* This is the top widget, that is the parent for all widgets and represents the screen. */
    pub obj: Obj,

    /* Widget that represents the top header area. */
    pub header: Obj,

    /* Widget that represents the bottom notification area. */
    pub notification: Obj,
    /* Text shown in notification area. */
    message_label: Obj,
}

impl Screen {
    /// Create a new screen with a header, let `content` add the widgets of the screen,
    /// then add the notification area below them.
    pub fn create<T>(config: &Config, content: impl FnOnce(Obj) -> T) -> (Screen, T) {
        let obj = Obj::new_screen();
        styles::apply_container_style_borderless(obj);
        obj.disable_scrolling();
        obj.set_style_pad_row(0, sys::LV_PART_MAIN);
        obj.set_size(lv::hor_res(), lv::ver_res());

        let header = Obj::new(obj);
        styles::apply_header_style(header);
        header.set_size(lv::pct(100), HEADER_HEIGHT);
        header.disable_scrolling();

        let title = Obj::new_label(header);
        title.label_set_text(&format!("{} {}", config.app_id, config.version));
        title.set_align(sys::LV_ALIGN_CENTER);

        let contents = content(obj);

        let notification = Obj::new(obj);
        notification.set_size(lv::pct(100), HEADER_HEIGHT);
        notification.set_flex_flow(sys::LV_FLEX_FLOW_ROW);
        notification.set_flex_align(
            sys::LV_FLEX_ALIGN_CENTER,
            sys::LV_FLEX_ALIGN_CENTER,
            sys::LV_FLEX_ALIGN_CENTER,
        );
        notification.disable_scrolling();

        let message_label = Obj::new_label(notification);
        message_label.label_set_long_mode(lv::LABEL_LONG_DOT);
        message_label.set_flex_grow(1);

        let screen = Screen {
            obj,
            header,
            notification,
            message_label,
        };
        (screen, contents)
    }

    pub fn load(&self) {
        self.obj.load_screen();
    }

    pub fn show_notification(&self, text: &str, kind: NotifyType) {
        let prefix = match kind {
            NotifyType::Info => "info",
            NotifyType::Success => "success",
            NotifyType::Warning => {
                log::warn!("warning notification: {text}");
                "warning"
            }
            NotifyType::Error => {
                log::error!("error notification: {text}");
                "error"
            }
        };

        styles::apply_notify_style(self.notification, kind);
        self.message_label.label_set_text(&format!("{prefix}: {text}"));
    }

    pub fn clear_notification(&self) {
        self.message_label.label_set_text("info: Ready");
        styles::apply_notify_style(self.notification, NotifyType::Info);
    }
}

/// Create a button with a text label.
pub fn create_button(parent: Obj, text: &'static std::ffi::CStr) -> (Obj, Obj) {
    let button = Obj::new_button(parent);
    styles::apply_button_style(button);

    let label = Obj::new_label(button);
    label.label_set_text_static(text);
    label.center();

    (button, label)
}
