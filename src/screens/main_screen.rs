// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains functions to create and operate main screen UI

use super::{create_button, file_browser, network_setup, screen_state, Screen};
use crate::config::{self, Config};
use crate::lv::{self, sys, GridNav, Obj};
use crate::networking::{self, Ipv4Info};
use crate::styles::{self, NotifyType};
use std::process::Command;

struct MainScreen {
    screen: Screen,

    /* Buttons for install-update, edit network-settings & restart-device operations */
    actions: Obj,
    install_button: Obj,
    network_setup_button: Obj,
    restart_button: Obj,

    /* Area to show network-information for available interfaces */
    network_info: Obj,
    interface_list: Option<Obj>,

    interfaces: Vec<Ipv4Info>,
}

screen_state!(MainScreen);

extern "C" fn on_click_install(_: *mut sys::lv_event_t) {
    file_browser::show();
}

extern "C" fn on_click_network_setup(_: *mut sys::lv_event_t) {
    network_setup::show();
}

extern "C" fn on_click_restart(_: *mut sys::lv_event_t) {
    let rebooted = Command::new("reboot").status().is_ok_and(|s| s.success());
    if !rebooted {
        with(|main| {
            main.screen
                .show_notification("Error rebooting the device", NotifyType::Error)
        });
    }
}

fn create_action_button(parent: Obj, text: &'static std::ffi::CStr, cb: lv::EventCb) -> Obj {
    let (button, _) = create_button(parent, text);
    button.set_size(lv::pct(100), 40);
    button.on(sys::LV_EVENT_CLICKED, cb, 0);
    button
}

/// Create recovery main screen UI
pub fn create(config: &Config) {
    let (screen, (actions, network_info)) = Screen::create(config, |obj| {
        let actions = Obj::new(obj);
        styles::apply_container_style_borderless(actions);
        actions.disable_scrolling();

        let network_info = Obj::new(obj);
        styles::apply_container_style_network_panel(network_info);
        network_info.disable_scrolling();

        (actions, network_info)
    });

    let install_button = create_action_button(actions, c"INSTALL FROM FILE", on_click_install);
    let network_setup_button =
        create_action_button(actions, c"NETWORK SETUP", on_click_network_setup);
    network_setup_button.set_hidden(true);
    let restart_button = create_action_button(actions, c"RESTART", on_click_restart);

    let main = MainScreen {
        screen,
        actions,
        install_button,
        network_setup_button,
        restart_button,
        network_info,
        interface_list: None,
        interfaces: Vec::new(),
    };
    main.set_layout();

    install(main);
}

/// Show main screen
pub fn show() {
    with(MainScreen::show);
}

impl MainScreen {
    fn show(&mut self) {
        self.refresh_interfaces();
        self.enable_navigation_via_keys();
        self.screen.clear_notification();
        self.screen.load();
    }

    fn set_layout(&self) {
        let obj = self.screen.obj;
        obj.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);
        obj.set_flex_align(
            sys::LV_FLEX_ALIGN_START,
            sys::LV_FLEX_ALIGN_CENTER,
            sys::LV_FLEX_ALIGN_SPACE_AROUND,
        );

        self.screen.header.set_flex_grow(0);

        // the panel with buttons
        self.actions.set_width(lv::pct(40));
        self.actions.set_flex_grow(1);
        self.actions.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);

        // the area with network information
        self.network_info.set_size(lv::pct(100), lv::pct(20));
        self.network_info.set_flex_grow(0);

        self.screen.notification.set_flex_grow(0);
    }

    fn enable_navigation_via_keys(&self) {
        lv::set_navigation(&[(self.actions, GridNav::Rollover)]);

        self.restart_button.remove_state(sys::LV_STATE_FOCUS_KEY);
        self.network_setup_button
            .remove_state(sys::LV_STATE_FOCUS_KEY);
        self.install_button.add_state(sys::LV_STATE_FOCUS_KEY);
    }

    fn refresh_interfaces(&mut self) {
        let config = &config::recovery_parameters().config;
        self.interfaces = networking::interface_list(&config.interface_names());

        if let Some(list) = self.interface_list.take() {
            list.delete();
        }

        if self.interfaces.is_empty() {
            self.network_setup_button.set_hidden(true);
        } else {
            self.create_network_info_list();
            self.network_setup_button.set_hidden(false);
        }
    }

    fn create_network_info_list(&mut self) {
        let list = Obj::new_list(self.network_info);
        styles::apply_network_list_style(list);
        list.set_size(lv::pct(100), lv::pct(100));

        // the rows of network interfaces plus one for the default gateway
        // are equally distributed vertically
        let row_height = lv::pct(100 / (self.interfaces.len() as i32 + 1));

        let gateway = networking::default_gateway_info();
        for info in self.interfaces.iter().chain(std::iter::once(&gateway)) {
            create_network_info_row(list, info, row_height);
        }

        self.interface_list = Some(list);
    }
}

fn create_network_info_row(list: Obj, info: &Ipv4Info, height: i32) {
    let entry = list.list_add_text("");
    styles::apply_network_list_entries_style(entry);
    entry.set_size(lv::pct(100), height);

    let row = Obj::new(entry);
    styles::apply_network_list_entries_style(row);
    row.set_size(lv::pct(100), lv::pct(100));
    row.set_flex_flow(sys::LV_FLEX_FLOW_ROW);
    row.set_flex_align(
        sys::LV_FLEX_ALIGN_SPACE_EVENLY,
        sys::LV_FLEX_ALIGN_CENTER,
        sys::LV_FLEX_ALIGN_CENTER,
    );
    row.disable_scrolling();

    // interface name, ip address and netmask
    for text in [&info.name, &info.address, &info.netmask] {
        let label = Obj::new_label(row);
        label.label_set_text(text);
        label.label_set_long_mode(lv::LABEL_LONG_SCROLL_CIRCULAR);
        label.set_width(lv::pct(25));
    }
}
