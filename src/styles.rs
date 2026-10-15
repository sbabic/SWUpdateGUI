// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains functions for creating and applying gui styles

use crate::config;
use crate::lv::{self, sys, Obj, Style};
use crate::system;

struct Styles {
    header: &'static Style,
    button: &'static Style,
    button_pressed: &'static Style,
    button_highlight: &'static Style,
    container_default: &'static Style,
    container_borderless: &'static Style,
    container_edit_panel: &'static Style,
    container_network_panel: &'static Style,
    network_list: &'static Style,
    network_list_entries: &'static Style,
    scrollbar: &'static Style,
    checkbox_default: &'static Style,
    checkbox_checked: &'static Style,
    checkbox_selected: &'static Style,
    checkbox_disabled: &'static Style,
    ip_label: &'static Style,
    network_screen_list: &'static Style,
    notify_info: &'static Style,
    notify_success: &'static Style,
    notify_warning: &'static Style,
    notify_error: &'static Style,
    network_entry_default: &'static Style,
    network_entry_selected: &'static Style,
    text_area_log: &'static Style,
}

impl Styles {
    fn create() -> Styles {
        let theme_color = &config::recovery_parameters().config.theme_color;
        let primary = system::palette_from_name(theme_color).unwrap_or_else(|| {
            log::error!("Invalid palette-color value: {theme_color}.");
            log::error!(
                "Valid values are: orange, red, pink, purple, indigo, blue, cyan, teal, green, brown."
            );
            std::process::exit(-1);
        });

        let grey = sys::LV_PALETTE_GREY;

        let notify = |color| {
            let style = Style::new();
            style
                .radius(0)
                .bg_color(color)
                .border_color(lv::palette_main(grey));
            style
        };

        Styles {
            header: {
                let s = Style::new();
                s.border_opa(lv::OPA_TRANSP)
                    .radius(0)
                    .text_align(sys::LV_TEXT_ALIGN_CENTER)
                    .text_color(lv::color_white())
                    .bg_color(lv::palette_darken(grey, 3));
                s
            },
            button: {
                let s = Style::new();
                s.radius(1)
                    .bg_opa(lv::OPA_COVER)
                    .bg_color(lv::palette_main(primary))
                    .text_color(lv::color_white())
                    .pad_all(10);
                s
            },
            button_pressed: {
                let s = Style::new();
                s.translate_y(5);
                s
            },
            button_highlight: {
                let s = Style::new();
                s.outline_color(lv::color_white());
                s
            },
            container_default: {
                let s = Style::new();
                s.border_opa(100)
                    .border_width(2)
                    .border_color(lv::palette_darken(grey, 3))
                    .bg_color(lv::palette_main(grey))
                    .radius(0);
                s
            },
            container_borderless: {
                let s = Style::new();
                s.border_opa(lv::OPA_TRANSP)
                    .border_width(0)
                    .radius(0)
                    .bg_color(lv::palette_main(grey));
                s
            },
            container_edit_panel: {
                let s = Style::new();
                s.border_opa(lv::OPA_TRANSP).radius(0).pad_all(0);
                s
            },
            container_network_panel: {
                let s = Style::new();
                s.border_opa(lv::OPA_TRANSP)
                    .radius(0)
                    .pad_all(0)
                    .bg_color(lv::palette_darken(grey, 3));
                s
            },
            network_list: {
                let s = Style::new();
                s.bg_color(lv::palette_main(grey)).radius(0).border_width(0);
                s
            },
            network_list_entries: {
                let s = Style::new();
                s.bg_color(lv::palette_darken(grey, 3))
                    .radius(0)
                    .border_width(0)
                    .text_color(lv::color_white())
                    .text_font(lv::font_montserrat_14());
                s
            },
            scrollbar: {
                let s = Style::new();
                s.width(15)
                    .pad_right(5)
                    .radius(2)
                    .bg_opa(lv::OPA_COVER)
                    .bg_color(lv::palette_lighten(primary, 2))
                    .border_color(lv::palette_main(primary))
                    .border_width(2);
                s
            },
            checkbox_default: {
                let s = Style::new();
                s.pad_all(0)
                    .bg_color(lv::palette_lighten(primary, 4))
                    .bg_opa(lv::OPA_COVER)
                    .radius(2)
                    .border_color(lv::palette_darken(grey, 2));
                s
            },
            checkbox_checked: {
                let s = Style::new();
                s.bg_color(lv::palette_darken(primary, 2))
                    .bg_opa(lv::OPA_COVER)
                    .radius(2);
                s
            },
            checkbox_disabled: {
                let s = Style::new();
                s.bg_color(lv::palette_main(grey))
                    .bg_opa(lv::OPA_COVER)
                    .radius(2);
                s
            },
            checkbox_selected: {
                let s = Style::new();
                s.outline_color(lv::color_white());
                s
            },
            ip_label: {
                let s = Style::new();
                s.text_font(lv::font_montserrat_14());
                s
            },
            network_screen_list: {
                let s = Style::new();
                s.border_opa(lv::OPA_TRANSP)
                    .radius(0)
                    .text_align(sys::LV_TEXT_ALIGN_CENTER)
                    .text_color(lv::color_white())
                    .bg_color(lv::palette_darken(grey, 1))
                    .pad_all(0);
                s
            },
            network_entry_default: {
                let s = Style::new();
                s.bg_color(lv::palette_main(grey));
                s
            },
            network_entry_selected: {
                let s = Style::new();
                s.outline_color(lv::color_white())
                    .bg_color(lv::palette_lighten(grey, 2))
                    .text_color(lv::color_black());
                s
            },
            notify_info: notify(lv::palette_lighten(grey, 1)),
            notify_success: notify(lv::palette_lighten(sys::LV_PALETTE_LIGHT_GREEN, 1)),
            notify_warning: notify(lv::palette_lighten(sys::LV_PALETTE_YELLOW, 1)),
            notify_error: notify(lv::palette_lighten(sys::LV_PALETTE_RED, 1)),
            text_area_log: {
                let s = Style::new();
                s.radius(0)
                    .bg_color(lv::palette_lighten(grey, 1))
                    .pad_all(5);
                s
            },
        }
    }
}

thread_local! {
    static STYLES: &'static Styles = Box::leak(Box::new(Styles::create()));
}

fn styles() -> &'static Styles {
    STYLES.with(|s| *s)
}

const MAIN: u32 = sys::LV_PART_MAIN;

pub fn apply_header_style(header: Obj) {
    header.add_style(styles().header, MAIN);
}

pub fn apply_button_style(button: Obj) {
    let s = styles();
    button.add_style(s.button, MAIN);
    button.add_style(s.button_pressed, sys::LV_STATE_PRESSED);
    button.add_style(s.button_highlight, MAIN | sys::LV_STATE_FOCUS_KEY);
}

pub fn apply_container_style(container: Obj) {
    container.add_style(styles().container_default, MAIN);
}

pub fn apply_container_style_borderless(container: Obj) {
    container.add_style(styles().container_borderless, MAIN);
}

pub fn apply_container_style_edit_panel(container: Obj) {
    container.add_style(styles().container_edit_panel, MAIN);
}

pub fn apply_container_style_network_panel(container: Obj) {
    container.add_style(styles().container_network_panel, MAIN);
}

pub fn apply_network_list_style(list: Obj) {
    list.add_style(styles().network_list, MAIN);
}

pub fn apply_network_list_entries_style(entry: Obj) {
    entry.add_style(styles().network_list_entries, MAIN);
}

pub fn apply_scrollbar_style(panel: Obj) {
    panel.remove_style(None, sys::LV_PART_SCROLLBAR | sys::LV_STATE_ANY);
    panel.add_style(styles().scrollbar, sys::LV_PART_SCROLLBAR);
}

pub fn apply_checkbox_style(checkbox: Obj) {
    let s = styles();
    checkbox.add_style(
        s.checkbox_default,
        sys::LV_PART_INDICATOR | sys::LV_STATE_DEFAULT,
    );
    checkbox.add_style(
        s.checkbox_checked,
        sys::LV_PART_INDICATOR | sys::LV_STATE_CHECKED,
    );
    checkbox.add_style(
        s.checkbox_disabled,
        sys::LV_PART_INDICATOR | sys::LV_STATE_DISABLED,
    );
    checkbox.add_style(s.checkbox_selected, MAIN | sys::LV_STATE_FOCUS_KEY);
}

pub fn apply_ip_label_style(label: Obj) {
    label.add_style(styles().ip_label, MAIN);
}

pub fn apply_network_screen_list_style(list: Obj) {
    list.add_style(styles().network_screen_list, MAIN);
}

pub fn apply_network_entry_style(entry: Obj) {
    let s = styles();
    entry.add_style(s.network_entry_default, MAIN);
    entry.add_style(s.network_entry_selected, MAIN | sys::LV_STATE_FOCUS_KEY);
}

pub fn apply_text_log_style(text_area: Obj) {
    text_area.add_style(styles().text_area_log, MAIN);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyType {
    Info,
    Success,
    Warning,
    Error,
}

/// Apply the notification style of the given type, replacing the previous one.
pub fn apply_notify_style(obj: Obj, kind: NotifyType) {
    let s = styles();
    for style in [
        s.notify_info,
        s.notify_success,
        s.notify_warning,
        s.notify_error,
    ] {
        obj.remove_style(Some(style), MAIN);
    }

    let style = match kind {
        NotifyType::Info => s.notify_info,
        NotifyType::Success => s.notify_success,
        NotifyType::Warning => s.notify_warning,
        NotifyType::Error => s.notify_error,
    };
    obj.add_style(style, MAIN);
}
