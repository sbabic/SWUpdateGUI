// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains functions to create & operate update-progress screen

use super::{create_button, main_screen, screen_state, Screen};
use crate::config::{self, Config};
use crate::lv::{self, sys, GridNav, Obj};
use crate::styles::{self, NotifyType};
use crate::swupdate_client::{self, Progress};
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/* Progress bar with a title */
struct ProgressBar {
    label: Obj,
    slider: Obj,
}

struct ProgressScreen {
    screen: Screen,

    /* Shows current step and total number of steps, and progress of the current step.
       Either the progress bars or the log messages are shown. */
    progress_bars: Obj,
    steps_progress: ProgressBar,
    percent_progress: ProgressBar,

    log_messages: Obj,
    log_text_area: Obj,

    /* The progress screen contains following buttons:
            - OK - to close the progress screen and go back to the main screen
            - SHOW LOGS - to show logs from swupdate program after an image update
       When logs are shown, following buttons are displayed:
            - BACK - to go back to the progress screen
            - EXPORT LOGS - the export the logs to the usb */
    actions_panel: Obj,
    ok_button: Obj,
    show_logs_button: Obj,
    back_button: Obj,
    export_logs_button: Obj,
}

screen_state!(ProgressScreen);

extern "C" fn on_click_ok(_: *mut sys::lv_event_t) {
    main_screen::show();
}

extern "C" fn on_click_show_logs(_: *mut sys::lv_event_t) {
    with(|p| p.show_page(Page::LogMessages));
}

extern "C" fn on_click_back(_: *mut sys::lv_event_t) {
    with(|p| p.show_page(Page::ProgressBars));
}

extern "C" fn on_click_export_logs(_: *mut sys::lv_event_t) {
    with(ProgressScreen::export_logs);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    ProgressBars,
    LogMessages,
}

fn create_progress_bar(parent: Obj, text: &str) -> ProgressBar {
    let container = Obj::new(parent);
    styles::apply_container_style_borderless(container);
    container.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);
    container.set_width(lv::pct(100));
    container.disable_scrolling();

    let label = Obj::new_label(container);
    label.set_align(sys::LV_ALIGN_CENTER);
    label.label_set_text(text);

    let slider = Obj::new_slider(container);
    slider.set_size(lv::pct(100), 30);
    slider.remove_flag(sys::LV_OBJ_FLAG_CLICKABLE);

    ProgressBar { label, slider }
}

/// Create progress screen
pub fn create(config: &Config) {
    let (screen, (progress_bars, log_messages, actions_panel)) = Screen::create(config, |obj| {
        (Obj::new(obj), Obj::new(obj), Obj::new(obj))
    });

    /* progress bars */
    progress_bars.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);
    styles::apply_container_style_borderless(progress_bars);
    progress_bars.disable_scrolling();

    let steps_progress = create_progress_bar(progress_bars, "NUMBER OF STEPS(0/0)");
    let percent_progress = create_progress_bar(progress_bars, "CURRENT STEP(0%)");

    /* log messages */
    let log_text_area = Obj::new_textarea(log_messages);
    styles::apply_text_log_style(log_text_area);
    styles::apply_scrollbar_style(log_text_area);
    log_text_area.set_size(lv::pct(100), lv::pct(100));
    log_text_area.center();
    log_text_area.set_style_text_align(sys::LV_TEXT_ALIGN_LEFT, sys::LV_PART_MAIN);

    /* buttons */
    styles::apply_container_style(actions_panel);
    actions_panel.disable_scrolling();
    actions_panel.set_flex_flow(sys::LV_FLEX_FLOW_ROW);
    actions_panel.set_flex_align(
        sys::LV_FLEX_ALIGN_CENTER,
        sys::LV_FLEX_ALIGN_START,
        sys::LV_FLEX_ALIGN_CENTER,
    );

    let button = |text, cb| {
        let (button, _) = create_button(actions_panel, text);
        button.center();
        button.set_size(lv::pct(50), 40);
        button.on(sys::LV_EVENT_CLICKED, cb, 0);
        button
    };

    let progress = ProgressScreen {
        screen,
        progress_bars,
        steps_progress,
        percent_progress,
        log_messages,
        log_text_area,
        actions_panel,
        ok_button: button(c"OK", on_click_ok),
        show_logs_button: button(c"SHOW LOGS", on_click_show_logs),
        back_button: button(c"BACK", on_click_back),
        export_logs_button: button(c"EXPORT LOGS", on_click_export_logs),
    };
    progress.reset_progress_bars();

    install(progress);
}

/// Show progress screen
pub fn show() {
    with(|p| {
        p.screen.load();
        p.show_page(Page::ProgressBars);
        p.enable_navigation();
    });
}

/// Start showing a new update: reset all fields and show the progress screen
pub fn start_new_update() {
    with(|p| {
        p.log_text_area.textarea_set_text("");
        p.screen
            .show_notification("UPDATE IN PROGRESS", NotifyType::Info);
        p.set_buttons_enabled(false);
    });
    show();
}

/// Update progress on the progress screen with latest information
pub fn update_progress(message: &Progress) {
    log::debug!(
        "Update message: {}/{}   {}%",
        message.cur_step,
        message.nsteps,
        message.cur_percent
    );

    with(|p| p.update_progress(message));
}

/// Show update result and the swupdate logs after the image update finished
pub fn finish_update(success: bool) {
    let messages = swupdate_client::messages();

    with(|p| {
        p.log_text_area.textarea_set_text("");
        for message in &messages {
            p.log_text_area.textarea_add_text(message);
            p.log_text_area.textarea_add_text("\n");
        }

        if success {
            p.screen.show_notification("Success!", NotifyType::Success);
        } else {
            p.screen.show_notification("Update failed!", NotifyType::Error);
        }
        p.set_buttons_enabled(true);
    });
}

impl ProgressScreen {
    fn set_layout(&self) {
        let obj = self.screen.obj;
        obj.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);
        obj.set_flex_align(
            sys::LV_FLEX_ALIGN_START,
            sys::LV_FLEX_ALIGN_CENTER,
            sys::LV_FLEX_ALIGN_SPACE_AROUND,
        );

        /* header goes at the top, and keeps its size */
        self.screen.header.set_align(sys::LV_ALIGN_TOP_MID);
        self.screen.header.set_flex_grow(0);

        /* progress bars and log messages go in the middle. when one is shown, the other is hidden */
        for panel in [self.progress_bars, self.log_messages] {
            panel.align_to(self.screen.header, sys::LV_ALIGN_OUT_BOTTOM_MID, 0, 0);
            panel.set_flex_flow(sys::LV_FLEX_FLOW_COLUMN);
            panel.set_flex_grow(1);
            panel.set_width(lv::pct(100));
        }

        /* notification bar goes at the bottom, and keeps its size */
        self.screen.notification.set_align(sys::LV_ALIGN_BOTTOM_MID);
        self.screen.notification.set_flex_grow(0);

        /* above notification area goes area with buttons */
        self.actions_panel
            .align_to(self.screen.notification, sys::LV_ALIGN_OUT_TOP_MID, 0, 0);
        self.actions_panel.set_width(lv::pct(100));
        self.actions_panel.set_height(lv::pct(15));
        self.actions_panel.set_flex_grow(0);
    }

    fn show_page(&self, page: Page) {
        self.set_layout();

        let logs = page == Page::LogMessages;

        self.progress_bars.set_hidden(logs);
        self.log_messages.set_hidden(!logs);

        self.ok_button.set_hidden(logs);
        self.show_logs_button.set_hidden(logs);
        self.back_button.set_hidden(!logs);
        self.export_logs_button.set_hidden(!logs);
    }

    fn enable_navigation(&self) {
        lv::set_navigation(&[(self.actions_panel, GridNav::None)]);

        self.ok_button.add_state(sys::LV_STATE_FOCUS_KEY);
        self.show_logs_button.add_state(sys::LV_STATE_FOCUS_KEY);
    }

    fn set_buttons_enabled(&self, enabled: bool) {
        for button in [self.ok_button, self.show_logs_button] {
            if enabled {
                button.remove_state(sys::LV_STATE_DISABLED);
            } else {
                button.add_state(sys::LV_STATE_DISABLED);
            }
        }
    }

    fn reset_progress_bars(&self) {
        self.steps_progress.slider.slider_set_value(0);
        self.percent_progress.slider.slider_set_value(0);

        self.set_buttons_enabled(false);

        self.screen
            .show_notification("UPDATE IN PROGRESS", NotifyType::Info);
    }

    fn update_progress(&self, message: &Progress) {
        if message.nsteps == 0 {
            self.steps_progress.slider.slider_set_value(0);
            self.percent_progress.slider.slider_set_value(0);
            return;
        }

        let steps_percent = 100 * message.cur_step / message.nsteps;

        self.steps_progress.label.label_set_text(&format!(
            "NUMBER OF STEPS({}/{})",
            message.cur_step, message.nsteps
        ));
        self.percent_progress
            .label
            .label_set_text(&format!("CURRENT STEP({}%)", message.cur_percent));

        self.percent_progress
            .slider
            .slider_set_value(message.cur_percent as i32);
        self.steps_progress
            .slider
            .slider_set_value(steps_percent as i32);
    }

    fn export_logs(&mut self) {
        let mediapath = &config::recovery_parameters().config.mediapath;

        match export_swupdate_logs(Path::new(mediapath)) {
            Ok(path) => self.screen.show_notification(
                &format!("Logs written to {}.", path.display()),
                NotifyType::Success,
            ),
            Err(err) => {
                log::error!("Failed to export swupdate logs to {mediapath}: {err}");
                self.screen
                    .show_notification("Failed to export logs.", NotifyType::Error)
            }
        }
    }
}

/// Create a new file with a unique name "swupdate.XXXXXX.log" in `dir`.
fn create_unique_log_file(dir: &Path) -> io::Result<(PathBuf, File)> {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
        ^ u64::from(std::process::id());

    for attempt in 0..100u64 {
        let suffix = (seed.wrapping_add(attempt.wrapping_mul(0x9E37_79B9_7F4A_7C15))) % 1_000_000;
        let path = dir.join(format!("swupdate.{suffix:06}.log"));

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    }

    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "unable to generate a unique filename",
    ))
}

/// Save swupdate logs in `dir`, and return the path of the written file.
fn export_swupdate_logs(dir: &Path) -> io::Result<PathBuf> {
    let (path, mut file) = create_unique_log_file(dir)?;
    log::info!("Exporting swupdate messages to {}.", path.display());

    for message in swupdate_client::messages() {
        writeln!(file, "{message}")?;
    }
    file.sync_all()?;
    drop(file);

    /* force synchronization of disk caches */
    unsafe { libc::sync() };

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_log_files() {
        let dir = std::env::temp_dir().join(format!("recovery-gui-logs-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();

        let (first, _) = create_unique_log_file(&dir).unwrap();
        let (second, _) = create_unique_log_file(&dir).unwrap();

        assert_ne!(first, second);
        let name = first.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("swupdate.") && name.ends_with(".log"), "{name}");

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
