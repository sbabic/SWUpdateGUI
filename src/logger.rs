// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Minimal logger writing to stderr. The level can be set with the
//                 RECOVERY_GUI_LOG environment variable (error, warn, info, debug, trace).

use log::{LevelFilter, Log, Metadata, Record};
use std::io::Write;

struct StderrLogger;

impl Log for StderrLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let _ = writeln!(
                std::io::stderr(),
                "[{}] {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    let level = std::env::var("RECOVERY_GUI_LOG")
        .ok()
        .and_then(|l| l.parse::<LevelFilter>().ok())
        .unwrap_or(LevelFilter::Info);

    if log::set_logger(&StderrLogger).is_ok() {
        log::set_max_level(level);
    }
}
