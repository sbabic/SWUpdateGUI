// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Process progress messages, received from the swupdate IPC thread,
//                 in the GUI thread.

use crate::screens::progress;
use crate::swupdate_client::{
    Progress, DONE, DOWNLOAD, FAILURE, IDLE, PROGRESS, RUN, START, SUBPROCESS, SUCCESS,
};
use std::sync::mpsc::Receiver;

pub struct ProgressEvents {
    receiver: Receiver<Progress>,
    /* tells whether the image update is in progress */
    update_in_progress: bool,
}

impl ProgressEvents {
    pub fn new(receiver: Receiver<Progress>) -> Self {
        ProgressEvents {
            receiver,
            update_in_progress: false,
        }
    }

    fn start_new_update(&mut self) {
        progress::start_new_update();
        self.update_in_progress = true;
    }

    fn finish_update(&mut self, success: bool) {
        progress::finish_update(success);
        self.update_in_progress = false;
    }

    /// Process all pending messages from swupdate
    pub fn process(&mut self) {
        while let Ok(message) = self.receiver.try_recv() {
            match message.status {
                IDLE | DONE | SUBPROCESS => {}

                START => {
                    log::info!("swupdate: Start update, source: {}", message.source);
                    self.start_new_update();
                }

                RUN | DOWNLOAD | PROGRESS => {
                    if !self.update_in_progress {
                        self.start_new_update();
                    }
                    progress::update_progress(&message);
                }

                SUCCESS => {
                    log::info!("swupdate: Success");
                    self.finish_update(true);
                }

                FAILURE => {
                    log::info!("swupdate: Failure {}", message.info);
                    self.finish_update(false);
                }

                other => log::error!("Invalid update state {other}"),
            }
        }
    }
}
