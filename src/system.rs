// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains utility functions for system-features

use crate::lv::sys;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EnvSettings {
    pub screen_orientation_angle: u32,
}

/// Get settings values from environment
pub fn env_settings() -> EnvSettings {
    let screen_orientation_angle = std::env::var("SCREEN_ORIENTATION_ANGLE")
        .ok()
        .and_then(|angle| angle.trim().parse().ok())
        .unwrap_or(0);

    EnvSettings {
        screen_orientation_angle,
    }
}

/// Convert angle in degrees (0, 90, 180, 270) to the LVGL rotation
pub fn rotation_from_angle(angle: u32) -> Option<sys::lv_display_rotation_t> {
    match angle {
        0 => Some(sys::LV_DISPLAY_ROTATION_0),
        90 => Some(sys::LV_DISPLAY_ROTATION_90),
        180 => Some(sys::LV_DISPLAY_ROTATION_180),
        270 => Some(sys::LV_DISPLAY_ROTATION_270),
        _ => None,
    }
}

/// Convert a color name to the LVGL palette
pub fn palette_from_name(color: &str) -> Option<sys::lv_palette_t> {
    let palette = match color {
        "orange" => sys::LV_PALETTE_ORANGE,
        "red" => sys::LV_PALETTE_RED,
        "pink" => sys::LV_PALETTE_PINK,
        "purple" => sys::LV_PALETTE_PURPLE,
        "indigo" => sys::LV_PALETTE_INDIGO,
        "blue" => sys::LV_PALETTE_BLUE,
        "cyan" => sys::LV_PALETTE_CYAN,
        "teal" => sys::LV_PALETTE_TEAL,
        "green" => sys::LV_PALETTE_GREEN,
        "brown" => sys::LV_PALETTE_BROWN,
        _ => return None,
    };
    Some(palette)
}

/// Executes a script and returns its exit code. The program is terminated when the script
/// can not be started, is stopped by a signal, or returns a value not in `valid_returns`.
pub fn run_script(script: &str, args: &[String], valid_returns: &[i32]) -> i32 {
    let command = format!("{script} {}", args.join(" "));

    let status = match Command::new(script).args(args).status() {
        Ok(status) => status,
        Err(err) => {
            log::error!("The command ({command}) could not be executed: {err}");
            std::process::exit(1);
        }
    };

    let Some(code) = status.code() else {
        log::error!(
            "The command ({command}) was stopped by a signal ({:?})",
            status.signal()
        );
        std::process::exit(-1);
    };

    if !valid_returns.contains(&code) {
        log::error!("Command ({command}) returned an unsupported return value ({code})");
        std::process::exit(1);
    }

    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_valid() {
        assert_eq!(rotation_from_angle(0), Some(sys::LV_DISPLAY_ROTATION_0));
        assert_eq!(rotation_from_angle(90), Some(sys::LV_DISPLAY_ROTATION_90));
        assert_eq!(rotation_from_angle(180), Some(sys::LV_DISPLAY_ROTATION_180));
        assert_eq!(rotation_from_angle(270), Some(sys::LV_DISPLAY_ROTATION_270));
    }

    #[test]
    fn rotation_invalid() {
        assert_eq!(rotation_from_angle(3), None);
    }

    #[test]
    fn palette() {
        assert_eq!(palette_from_name("orange"), Some(sys::LV_PALETTE_ORANGE));
        assert_eq!(palette_from_name("brown"), Some(sys::LV_PALETTE_BROWN));
        assert_eq!(palette_from_name(""), None);
        assert_eq!(palette_from_name("dark-blue"), None);
    }

    #[test]
    fn run_script_return_value() {
        assert_eq!(
            run_script("/bin/sh", &["-c".into(), "exit 1".into()], &[0, 1]),
            1
        );
        assert_eq!(
            run_script("/bin/sh", &["-c".into(), "true".into()], &[0, 1]),
            0
        );
    }
}
