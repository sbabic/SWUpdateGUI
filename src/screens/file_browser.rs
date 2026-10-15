// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains functions to create and operate file browser screen

use super::{create_button, main_screen, progress, screen_state, Screen};
use crate::config::Config;
use crate::files::{self, DirInfo, FENTRIES_MAX};
use crate::lv::{self, sys, Event, GridNav, Obj};
use crate::styles::{self, NotifyType};
use crate::swupdate_client;
use std::path::Path;

struct FileBrowserScreen {
    screen: Screen,

    /* Dry-run checkbox */
    dry_run_panel: Obj,
    dry_run_checkbox: Obj,

    /* List of files/folders */
    contents_panel: Obj,
    dir_path_label: Obj,
    dir_contents_panel: Obj,
    entries: Vec<Obj>,

    /* Buttons for start/rescan/cancel actions */
    actions_panel: Obj,
    start_button: Obj,

    dir_info: DirInfo,
    /* index into dir_info.swu_files */
    selected_file: Option<usize>,
    current_dir: String,
    top_dir: String,
}

screen_state!(FileBrowserScreen);

extern "C" fn on_click_dir_entry(e: *mut sys::lv_event_t) {
    let index = unsafe { Event::from_raw(e) }.user_data();
    with(|browser| browser.enter_dir(index));
}

extern "C" fn on_click_file_entry(e: *mut sys::lv_event_t) {
    let event = unsafe { Event::from_raw(e) };
    let (index, checkbox) = (event.user_data(), event.target());
    with(|browser| {
        if let Some(checkbox) = checkbox {
            browser.select_file(index, checkbox);
        }
    });
}

extern "C" fn on_click_start(_: *mut sys::lv_event_t) {
    if let Some(Some((path, dry_run))) = with(FileBrowserScreen::selected_update) {
        progress::show();
        swupdate_client::start_local_update(&path, dry_run);
    }
}

extern "C" fn on_click_rescan(_: *mut sys::lv_event_t) {
    with(FileBrowserScreen::refresh_entries);
}

extern "C" fn on_click_cancel(_: *mut sys::lv_event_t) {
    with(FileBrowserScreen::delete_file_entries);
    main_screen::show();
}

/// Create file-browser screen
pub fn create(config: &Config) {
    let (screen, (dry_run_panel, contents_panel, actions_panel)) = Screen::create(config, |obj| {
        let dry_run_panel = Obj::new(obj);
        let contents_panel = Obj::new(obj);
        let actions_panel = Obj::new(obj);
        (dry_run_panel, contents_panel, actions_panel)
    });

    /* dry run checkbox */
    styles::apply_container_style_borderless(dry_run_panel);
    dry_run_panel.set_size(lv::pct(100), lv::pct(10));
    dry_run_panel.set_flex_flow(sys::LV_FLEX_FLOW_ROW);
    dry_run_panel.set_flex_align(
        sys::LV_FLEX_ALIGN_START,
        sys::LV_FLEX_ALIGN_CENTER,
        sys::LV_FLEX_ALIGN_SPACE_EVENLY,
    );

    let dry_run_checkbox = Obj::new_checkbox(dry_run_panel);
    styles::apply_checkbox_style(dry_run_checkbox);
    dry_run_checkbox.checkbox_set_text("Enable DRY-RUN");

    /* file list */
    styles::apply_container_style(contents_panel);
    contents_panel.disable_scrolling();
    contents_panel.set_size(lv::pct(100), lv::pct(55));
    contents_panel.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);

    let dir_path_label = Obj::new_label(contents_panel);
    dir_path_label.set_size(lv::pct(100), lv::pct(10));

    let dir_contents_panel = create_list_header(contents_panel);

    /* buttons */
    styles::apply_container_style_borderless(actions_panel);
    actions_panel.set_size(lv::pct(100), lv::pct(10));
    actions_panel.set_flex_flow(sys::LV_FLEX_FLOW_ROW);
    actions_panel.set_flex_align(
        sys::LV_FLEX_ALIGN_SPACE_EVENLY,
        sys::LV_FLEX_ALIGN_CENTER,
        sys::LV_FLEX_ALIGN_SPACE_EVENLY,
    );
    actions_panel.disable_scrolling();

    let action_button = |text, cb| {
        let (button, _) = create_button(actions_panel, text);
        button.set_size(lv::pct(33), 50);
        button.on(sys::LV_EVENT_CLICKED, cb, 0);
        button
    };
    let start_button = action_button(c"START", on_click_start);
    action_button(c"RESCAN", on_click_rescan);
    action_button(c"CANCEL", on_click_cancel);

    let browser = FileBrowserScreen {
        screen,
        dry_run_panel,
        dry_run_checkbox,
        contents_panel,
        dir_path_label,
        dir_contents_panel,
        entries: Vec::new(),
        actions_panel,
        start_button,
        dir_info: DirInfo::default(),
        selected_file: None,
        current_dir: config.mediapath.clone(),
        top_dir: config.mediapath.clone(),
    };
    browser.set_start_button_state(false);
    browser.set_layout();

    install(browser);
}

/// Load file-browser screen
pub fn show() {
    with(FileBrowserScreen::show);
}

fn create_list_header(parent: Obj) -> Obj {
    let panel = Obj::new(parent);
    panel.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);
    panel.set_flex_align(
        sys::LV_FLEX_ALIGN_START,
        sys::LV_FLEX_ALIGN_CENTER,
        sys::LV_FLEX_ALIGN_SPACE_AROUND,
    );
    styles::apply_container_style(panel);
    styles::apply_scrollbar_style(panel);
    panel.set_size(lv::pct(100), lv::pct(90));

    let row_header = Obj::new(panel);
    row_header.remove_flag(sys::LV_OBJ_FLAG_CLICK_FOCUSABLE);
    styles::apply_container_style_borderless(row_header);
    row_header.set_style_pad_all(0, sys::LV_PART_MAIN);
    row_header.set_size(lv::pct(100), 30);
    row_header.set_flex_flow(sys::LV_FLEX_FLOW_ROW);
    row_header.set_flex_align(
        sys::LV_FLEX_ALIGN_START,
        sys::LV_FLEX_ALIGN_CENTER,
        sys::LV_FLEX_ALIGN_SPACE_EVENLY,
    );

    Obj::new_label(row_header).label_set_text("FILENAME");
    Obj::new_label(row_header).label_set_text("(SIZE in Bytes)");

    panel
}

impl FileBrowserScreen {
    fn show(&mut self) {
        self.screen.clear_notification();

        self.current_dir = self.top_dir.clone();
        self.dir_path_label.label_set_text(&self.current_dir);

        self.dry_run_checkbox.remove_state(sys::LV_STATE_CHECKED);

        self.refresh_entries();

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
        self.dry_run_panel.set_flex_grow(0);
        self.contents_panel.set_flex_grow(1);
        self.actions_panel.set_flex_grow(0);
        self.screen.notification.set_flex_grow(0);
    }

    fn set_start_button_state(&self, enabled: bool) {
        let button = self.start_button;
        let flags = sys::LV_OBJ_FLAG_CLICK_FOCUSABLE | sys::LV_OBJ_FLAG_CLICKABLE;

        if enabled {
            button.remove_state(sys::LV_STATE_DISABLED);
            button.add_flag(flags);
        } else {
            button.add_state(sys::LV_STATE_DISABLED);
            button.remove_flag(flags);
        }
    }

    /// Path of the selected file and the dry-run setting, if a file is selected.
    fn selected_update(&mut self) -> Option<(String, bool)> {
        let file = &self.dir_info.swu_files[self.selected_file?];
        let path = Path::new(&self.current_dir).join(&file.name);
        let dry_run = self.dry_run_checkbox.has_state(sys::LV_STATE_CHECKED);

        Some((path.to_string_lossy().into_owned(), dry_run))
    }

    fn select_file(&mut self, index: usize, checkbox: Obj) {
        if checkbox.has_state(sys::LV_STATE_CHECKED) {
            self.selected_file = Some(index);

            for entry in self.entries.iter().filter(|&&e| e != checkbox) {
                entry.remove_state(sys::LV_STATE_CHECKED);
            }
            self.set_start_button_state(true);
        } else {
            self.selected_file = None;
            self.set_start_button_state(false);
        }
    }

    fn enter_dir(&mut self, index: usize) {
        let Some(dir) = self.dir_info.dirs.get(index).cloned() else {
            return;
        };

        if !files::update_current_dir(&mut self.current_dir, &dir) {
            self.screen.show_notification(
                "The selected directory path is not allowed.",
                NotifyType::Error,
            );
            return;
        }
        self.dir_path_label.label_set_text(&self.current_dir);

        self.screen.clear_notification();
        self.refresh_entries();
    }

    fn delete_file_entries(&mut self) {
        for entry in self.entries.drain(..) {
            entry.delete();
        }
        self.dir_info = DirInfo::default();
        self.selected_file = None;
    }

    /// Re-read the current directory and recreate the list of entries
    fn refresh_entries(&mut self) {
        self.delete_file_entries();
        self.set_start_button_state(false);
        self.refresh_dir_info();
        self.create_file_entries();
        self.enable_navigation_via_keys();
    }

    fn refresh_dir_info(&mut self) {
        self.dir_info = files::list_all_swu_files(Path::new(&self.current_dir));

        let count = self.dir_info.entry_count();
        if count > FENTRIES_MAX {
            self.screen.show_notification(
                &format!(
                    "Too many Files/Directories: {count}. Only first {FENTRIES_MAX} entries will be visible."
                ),
                NotifyType::Warning,
            );
            self.dir_info.truncate(FENTRIES_MAX);
        }

        self.dir_info
            .remove_parent_entry(&self.top_dir, &self.current_dir);

        if self.dir_info.swu_files.is_empty() {
            self.screen
                .show_notification("No swu files available!", NotifyType::Warning);
        }
    }

    fn create_file_entries(&mut self) {
        for (index, dir) in self.dir_info.dirs.iter().enumerate() {
            if !files::is_valid_dir_name_length(dir) {
                self.screen
                    .show_notification("The directory name is too long.", NotifyType::Warning);
                continue;
            }

            let button = Obj::new_button(self.dir_contents_panel);
            styles::apply_button_style(button);
            button.set_size(lv::pct(100), 40);

            let label = Obj::new_label(button);
            label.align(sys::LV_ALIGN_LEFT_MID, 0, 0);
            label.label_set_text(dir);

            button.on(sys::LV_EVENT_CLICKED, on_click_dir_entry, index);
            self.entries.push(button);
        }

        for (index, file) in self.dir_info.swu_files.iter().enumerate() {
            let Some(text) = files::file_entry_label(&file.name, file.size) else {
                self.screen
                    .show_notification("Filename is too long.", NotifyType::Warning);
                continue;
            };

            let checkbox = Obj::new_checkbox(self.dir_contents_panel);
            styles::apply_checkbox_style(checkbox);
            checkbox.set_width(lv::pct(100));
            checkbox.checkbox_set_text(&text);

            checkbox.on(sys::LV_EVENT_CLICKED, on_click_file_entry, index);
            self.entries.push(checkbox);
        }

        self.selected_file = None;
    }

    fn enable_navigation_via_keys(&self) {
        let mut containers = vec![(self.dry_run_panel, GridNav::ScrollFirst)];
        if !self.entries.is_empty() {
            containers.push((self.dir_contents_panel, GridNav::ScrollFirst));
        }
        containers.push((self.actions_panel, GridNav::ScrollFirst));

        lv::set_navigation(&containers);

        self.dry_run_checkbox.add_state(sys::LV_STATE_FOCUS_KEY);
    }
}
