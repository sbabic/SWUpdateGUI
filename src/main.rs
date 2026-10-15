// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Recovery GUI for SWUpdate: install updates from local media and edit
//                 the network settings.

mod config;
mod ffi;
mod files;
mod logger;
mod lv;
mod lvgl_init;
mod networking;
mod progress_events;
mod screens;
mod styles;
mod swupdate_client;
mod system;

use progress_events::ProgressEvents;
use std::sync::mpsc;

fn main() {
    logger::init();

    let params = config::recovery_parameters();

    lvgl_init::initialize(params);

    let (sender, receiver) = mpsc::channel();
    swupdate_client::start_ipc_thread(sender);

    screens::create_all(&params.config);
    screens::main_screen::show();

    let mut events = ProgressEvents::new(receiver);
    lvgl_init::run(|| events.process());
}
