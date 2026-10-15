// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains functions to edit network settings
//
// The screen has two pages:
//   1. the list of network interfaces and the default gateway, with OK and EDIT buttons
//   2. the edit area for the selected entry with a virtual keyboard, either with
//      SET STATIC, SET DHCP and CANCEL buttons for an interface, or
//      SET, DELETE and CANCEL buttons for the default gateway

use super::{create_button, main_screen, screen_state, Screen};
use crate::config::{self, Config};
use crate::lv::{self, sys, Event, GridNav, Obj};
use crate::networking::{self, Ipv4Info};
use crate::styles::{self, NotifyType};
use std::ffi::CStr;

const BUTTON_COUNT: usize = 5;

/* What a button of the actions panel does on the current page */
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Action {
    None,
    Ok,
    Edit,
    SetStatic,
    SetDhcp,
    SetGateway,
    DeleteGateway,
    Cancel,
}

/* One row of the interface table */
struct TableRow {
    checkbox: Obj,
}

/* Label and text area to edit an ip address or netmask */
struct AddressPanel {
    container: Obj,
    text_area: Obj,
}

struct NetworkScreen {
    screen: Screen,

    /* panel showing list of network interfaces: a table of N+1 rows, where N is the number of
    network interfaces and the last row is the default gateway. Each row shows the name,
    ip address, netmask and whether it is configured as a DHCP client. */
    list_panel: Obj,
    table: Obj,
    rows: Vec<TableRow>,
    interfaces: Vec<Ipv4Info>,
    default_gateway: Ipv4Info,
    /* index into interfaces, or interfaces.len() for the default gateway */
    selected: Option<usize>,

    /* panel to edit settings for the selected network interface or default gateway */
    edit_panel: Obj,
    selected_interface_label: Obj,
    ip_address: AddressPanel,
    netmask: AddressPanel,
    keyboard: Obj,

    /* action area holding buttons */
    actions: Obj,
    buttons: [Obj; BUTTON_COUNT],
    labels: [Obj; BUTTON_COUNT],
    button_actions: [Action; BUTTON_COUNT],
}

screen_state!(NetworkScreen);

extern "C" fn on_click_action_button(e: *mut sys::lv_event_t) {
    let index = unsafe { Event::from_raw(e) }.user_data();

    // Returning to the main screen needs the main screen state, not this one.
    if with(|n| n.on_action(index)) == Some(true) {
        main_screen::show();
    }
}

extern "C" fn on_click_table_row(e: *mut sys::lv_event_t) {
    let index = unsafe { Event::from_raw(e) }.user_data();
    with(|n| n.toggle_row(index));
}

extern "C" fn on_click_text_area(e: *mut sys::lv_event_t) {
    if let Some(text_area) = unsafe { Event::from_raw(e) }.current_target() {
        with(|n| n.edit_text_area(text_area));
    }
}

/// Create network-settings screen
pub fn create(config: &Config) {
    let (screen, (container, actions_panel)) =
        Screen::create(config, |obj| (Obj::new(obj), Obj::new(obj)));

    /* contents: interface list and edit area, with one of them visible */
    container.set_width(lv::pct(100));
    styles::apply_container_style_borderless(container);
    container.disable_scrolling();
    container.set_flex_grow(2);
    container.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);

    let list_panel = Obj::new(container);
    styles::apply_container_style_borderless(list_panel);
    list_panel.set_size(lv::pct(100), lv::pct(100));
    list_panel.disable_scrolling();
    list_panel.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);
    let table = create_table(list_panel);

    let edit_panel = Obj::new(container);
    styles::apply_container_style(edit_panel);
    edit_panel.set_size(lv::pct(100), lv::pct(100));
    edit_panel.disable_scrolling();
    edit_panel.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);

    let selected_interface_label = Obj::new_label(edit_panel);
    selected_interface_label.set_size(lv::pct(100), 30);

    let address_row = Obj::new(edit_panel);
    styles::apply_container_style_borderless(address_row);
    address_row.set_size(lv::pct(100), 80);
    address_row.set_flex_flow(sys::LV_FLEX_FLOW_ROW);
    address_row.set_style_pad_all(0, sys::LV_PART_MAIN);
    address_row.disable_scrolling();

    let ip_address = create_address_panel(address_row, "IP Address", "aaa.aaa.aaa.aaa");
    let netmask = create_address_panel(address_row, "Netmask", "nnn.nnn.nnn.nnn");

    let keyboard = Obj::new_keyboard(edit_panel);
    keyboard.set_flex_grow(1);
    keyboard.keyboard_set_mode(sys::LV_KEYBOARD_MODE_NUMBER);

    /* buttons */
    styles::apply_container_style_borderless(actions_panel);
    actions_panel.set_size(lv::pct(100), lv::pct(10));
    actions_panel.set_style_pad_all(0, sys::LV_PART_MAIN);
    actions_panel.disable_scrolling();

    let actions = Obj::new(actions_panel);
    styles::apply_container_style_borderless(actions);
    actions.set_size(lv::pct(100), lv::pct(100));
    actions.set_flex_flow(sys::LV_FLEX_FLOW_ROW);
    actions.set_flex_align(
        sys::LV_FLEX_ALIGN_CENTER,
        sys::LV_FLEX_ALIGN_START,
        sys::LV_FLEX_ALIGN_CENTER,
    );
    actions.disable_scrolling();

    let mut buttons = [actions; BUTTON_COUNT];
    let mut labels = [actions; BUTTON_COUNT];
    for (index, (button, label)) in buttons.iter_mut().zip(labels.iter_mut()).enumerate() {
        (*button, *label) = create_button(actions, c"");
        button.set_height(40);
        button.set_width(lv::pct(33));
        button.on(sys::LV_EVENT_CLICKED, on_click_action_button, index);
    }

    let network = NetworkScreen {
        screen,
        list_panel,
        table,
        rows: Vec::new(),
        interfaces: Vec::new(),
        default_gateway: networking::default_gateway_info(),
        selected: None,
        edit_panel,
        selected_interface_label,
        ip_address,
        netmask,
        keyboard,
        actions,
        buttons,
        labels,
        button_actions: [Action::None; BUTTON_COUNT],
    };
    network.set_layout();

    install(network);
}

/// Show network-settings screen
pub fn show() {
    with(|n| {
        n.screen.clear_notification();
        n.repopulate_table();
        n.show_interface_list_page();
        n.screen.load();
    });
}

fn create_table(parent: Obj) -> Obj {
    let table = Obj::new_list(parent);
    styles::apply_network_screen_list_style(table);
    styles::apply_scrollbar_style(table);
    table.set_width(lv::pct(100));
    table.set_flex_grow(5);

    /* the header row */
    let header = table.list_add_button();
    header.set_width(lv::pct(100));
    header.remove_flag(sys::LV_OBJ_FLAG_CLICKABLE | sys::LV_OBJ_FLAG_CLICK_FOCUSABLE);
    header.add_state(sys::LV_STATE_DISABLED);
    header.set_style_bg_color(lv::palette_main(sys::LV_PALETTE_GREY), sys::LV_PART_MAIN);
    set_row_layout(header);

    for text in ["NETIF", "IP ADDRESS", "NETMASK", "DHCP"] {
        create_column_label(header, text);
    }

    table
}

fn set_row_layout(row: Obj) {
    row.set_flex_flow(sys::LV_FLEX_FLOW_ROW);
    row.set_flex_align(
        sys::LV_FLEX_ALIGN_START,
        sys::LV_FLEX_ALIGN_CENTER,
        sys::LV_FLEX_ALIGN_SPACE_EVENLY,
    );
}

fn create_column_label(row: Obj, text: &str) -> Obj {
    let label = Obj::new_label(row);
    label.set_width(lv::pct(25));
    label.set_style_text_align(sys::LV_TEXT_ALIGN_LEFT, sys::LV_PART_MAIN);
    label.label_set_text(text);
    label
}

fn create_table_row(table: Obj, info: &Ipv4Info, index: usize) -> TableRow {
    let row = table.list_add_button();
    row.set_width(lv::pct(100));
    set_row_layout(row);
    row.on(sys::LV_EVENT_CLICKED, on_click_table_row, index);
    styles::apply_network_entry_style(row);

    /* The checkbox shows the selection; clicks are handled by the row */
    let checkbox = Obj::new_checkbox(row);
    checkbox.set_width(lv::pct(25));
    checkbox.checkbox_set_text(&info.name);
    checkbox.add_flag(sys::LV_OBJ_FLAG_EVENT_BUBBLE);
    checkbox.remove_flag(sys::LV_OBJ_FLAG_CLICKABLE);

    for text in [
        info.address.as_str(),
        info.netmask.as_str(),
        info.dhcp_text(),
    ] {
        let label = create_column_label(row, text);
        label.add_flag(sys::LV_OBJ_FLAG_CLICKABLE | sys::LV_OBJ_FLAG_EVENT_BUBBLE);
    }

    TableRow { checkbox }
}

fn create_address_panel(parent: Obj, label_text: &str, placeholder: &str) -> AddressPanel {
    let container = Obj::new(parent);
    container.set_width(lv::pct(50));
    styles::apply_container_style_edit_panel(container);
    container.set_height(lv::pct(100));
    container.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);
    container.disable_scrolling();
    container.remove_flag(sys::LV_OBJ_FLAG_CLICK_FOCUSABLE);

    let label = Obj::new_label(container);
    styles::apply_ip_label_style(label);
    label.set_width(lv::pct(100));
    label.label_set_text(label_text);

    let text_area = Obj::new_textarea(container);
    text_area.set_width(lv::pct(75));
    text_area.set_height(40);
    text_area.textarea_set_one_line(true);
    text_area.textarea_set_cursor_click_pos(false);
    text_area.textarea_set_placeholder_text(placeholder);
    text_area.on(sys::LV_EVENT_CLICKED, on_click_text_area, 0);

    AddressPanel {
        container,
        text_area,
    }
}

impl NetworkScreen {
    /*
     * Set the screen layout:
     *      - header at the top
     *      - interface list and edit area in the middle bellow the header
     *      - action area with buttons above notification area
     *      - notification area at the bottom
     */
    fn set_layout(&self) {
        let obj = self.screen.obj;
        obj.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);
        obj.set_flex_align(
            sys::LV_FLEX_ALIGN_START,
            sys::LV_FLEX_ALIGN_CENTER,
            sys::LV_FLEX_ALIGN_SPACE_AROUND,
        );

        self.screen.header.align(sys::LV_ALIGN_TOP_MID, 0, 0);
        self.list_panel
            .align_to(self.screen.header, sys::LV_ALIGN_OUT_BOTTOM_MID, 0, 0);
        self.edit_panel
            .align_to(self.screen.header, sys::LV_ALIGN_OUT_BOTTOM_MID, 0, 0);
        self.screen
            .notification
            .align(sys::LV_ALIGN_BOTTOM_MID, 0, 0);
        self.actions
            .align_to(self.screen.notification, sys::LV_ALIGN_OUT_TOP_MID, 0, 0);
    }

    fn item(&self, index: usize) -> &Ipv4Info {
        self.interfaces.get(index).unwrap_or(&self.default_gateway)
    }

    /// Update the list of network interfaces and recreate the table
    fn repopulate_table(&mut self) {
        let config = &config::recovery_parameters().config;

        self.selected = None;
        self.interfaces = networking::interface_list(&config.interface_names());
        self.default_gateway = networking::default_gateway_info();

        self.table.delete();
        self.table = create_table(self.list_panel);

        let items = self
            .interfaces
            .iter()
            .chain(std::iter::once(&self.default_gateway));
        self.rows = items
            .enumerate()
            .map(|(index, info)| create_table_row(self.table, info, index))
            .collect();
    }

    fn clear_all_checkboxes(&self) {
        for row in &self.rows {
            row.checkbox.remove_state(sys::LV_STATE_CHECKED);
        }
    }

    fn toggle_row(&mut self, index: usize) {
        let Some(checkbox) = self.rows.get(index).map(|r| r.checkbox) else {
            return;
        };

        if checkbox.has_state(sys::LV_STATE_CHECKED) {
            self.selected = None;
            checkbox.remove_state(sys::LV_STATE_CHECKED);
        } else {
            self.clear_all_checkboxes();
            checkbox.add_state(sys::LV_STATE_CHECKED);
            self.selected = Some(index);
        }
    }

    fn edit_text_area(&self, text_area: Obj) {
        self.keyboard.keyboard_set_textarea(Some(text_area));

        for panel in [&self.ip_address, &self.netmask] {
            panel
                .text_area
                .set_style_bg_color(lv::color_white(), sys::LV_PART_MAIN);
        }
        text_area.set_style_bg_color(lv::palette_main(sys::LV_PALETTE_YELLOW), sys::LV_PART_MAIN);

        lv::group_focus(text_area);
    }

    /// Configure the buttons of the actions panel; unused buttons are hidden.
    fn configure_buttons(&mut self, buttons: &[(usize, &'static CStr, Action)]) {
        for (button, action) in self.buttons.iter().zip(self.button_actions.iter_mut()) {
            button.set_hidden(true);
            *action = Action::None;
        }

        for &(index, text, action) in buttons {
            self.buttons[index].set_hidden(false);
            self.labels[index].label_set_text_static(text);
            self.button_actions[index] = action;
        }
    }

    fn show_interface_list_page(&mut self) {
        self.list_panel.set_hidden(false);
        self.edit_panel.set_hidden(true);

        self.configure_buttons(&[(0, c"OK", Action::Ok), (1, c"EDIT", Action::Edit)]);

        lv::set_navigation(&[
            (self.table, GridNav::ScrollFirst),
            (self.actions, GridNav::None),
        ]);

        self.clear_all_checkboxes();
        self.selected = None;
    }

    fn show_edit_page(&mut self) {
        let Some(selected) = self.selected else {
            self.screen.show_notification(
                "Please select an interface for editing first!",
                NotifyType::Warning,
            );
            return;
        };

        let item = self.item(selected).clone();

        self.selected_interface_label.label_set_text(&item.name);
        for (panel, text) in [
            (&self.ip_address, &item.address),
            (&self.netmask, &item.netmask),
        ] {
            panel
                .text_area
                .set_style_bg_color(lv::color_white(), sys::LV_PART_MAIN);
            panel.text_area.textarea_set_text(text);
        }
        self.keyboard.keyboard_set_textarea(None);

        self.list_panel.set_hidden(true);
        self.edit_panel.set_hidden(false);

        let is_gateway = item.is_default_gateway();
        self.netmask.container.set_hidden(is_gateway);

        let mut navigation = vec![(self.ip_address.container, GridNav::None)];
        if is_gateway {
            self.configure_buttons(&[
                (2, c"SET", Action::SetGateway),
                (3, c"DELETE", Action::DeleteGateway),
                (4, c"CANCEL", Action::Cancel),
            ]);
        } else {
            self.configure_buttons(&[
                (2, c"SET STATIC", Action::SetStatic),
                (3, c"SET DHCP", Action::SetDhcp),
                (4, c"CANCEL", Action::Cancel),
            ]);
            navigation.push((self.netmask.container, GridNav::None));
        }
        navigation.push((self.keyboard, GridNav::None));
        navigation.push((self.actions, GridNav::None));
        lv::set_navigation(&navigation);
    }

    fn notify_result(&self, success: bool, ok: &str, failed: &str) {
        if success {
            self.screen.show_notification(ok, NotifyType::Success);
        } else {
            self.screen.show_notification(failed, NotifyType::Error);
        }
    }

    fn selected_name(&self) -> Option<String> {
        self.selected.map(|index| self.item(index).name.clone())
    }

    /// Handle a click on an action button. Returns true when the main screen has to be shown.
    fn on_action(&mut self, button: usize) -> bool {
        let Some(&action) = self.button_actions.get(button) else {
            return false;
        };

        match action {
            Action::None => {}
            Action::Ok => return true,
            Action::Edit => self.show_edit_page(),
            Action::Cancel => self.show_interface_list_page(),
            Action::SetStatic => self.set_static(),
            Action::SetDhcp => self.set_dhcp(),
            Action::SetGateway => self.set_gateway(),
            Action::DeleteGateway => self.delete_gateway(),
        }
        false
    }

    fn set_static(&mut self) {
        let Some(name) = self.selected_name() else {
            return;
        };
        let address = self.ip_address.text_area.textarea_text();
        let netmask = self.netmask.text_area.textarea_text();

        if address.is_empty() {
            self.screen
                .show_notification("The ip address can not be an empty.", NotifyType::Error);
        } else if netmask.is_empty() {
            self.screen
                .show_notification("The netmask can not be an empty.", NotifyType::Error);
        } else {
            let success = networking::set_static_configuration(&name, &address, &netmask);
            self.notify_result(
                success,
                &format!("Successfully applied static setting for '{name}'."),
                &format!("Failed to apply static settings for '{name}'."),
            );

            self.repopulate_table();
            self.show_interface_list_page();
        }
    }

    fn set_dhcp(&mut self) {
        let Some(name) = self.selected_name() else {
            return;
        };

        let success = networking::reconfigure_as_dhcp_client(&name);
        self.notify_result(
            success,
            &format!("Successfully reconfigured '{name}' as a dhcp client."),
            &format!("Failed to reconfigure '{name}' as a dhcp client."),
        );

        self.repopulate_table();
        self.show_interface_list_page();
    }

    fn set_gateway(&mut self) {
        let address = self.ip_address.text_area.textarea_text();

        if address.is_empty() {
            self.screen.show_notification(
                "The gateway address can not be an empty string.",
                NotifyType::Error,
            );
            return;
        }

        let success = networking::set_gateway_address(&address);
        self.notify_result(
            success,
            "Gateway set successfully.",
            "Failed to set the gateway address.",
        );
        if success {
            self.repopulate_table();
        }

        self.show_interface_list_page();
    }

    fn delete_gateway(&mut self) {
        let success = networking::delete_gateway();
        self.notify_result(
            success,
            "Gateway IP address deleted!",
            "Failed to delete gateway!",
        );
        if success {
            self.repopulate_table();
        }

        self.show_interface_list_page();
    }
}
