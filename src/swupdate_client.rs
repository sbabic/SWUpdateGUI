// SPDX-License-Identifier: GPL-2.0-or-later
//
// Copyright (c) 2024 IFM Ecomatic GmbH
// Copyright (c) 2026 Stefano Babic <stefano.babic@swupdate.org>
//
//  - Description: Contains functions to communicate to the SwUpdate process
//
// There are two threads involved: the GUI thread, and a thread receiving progress and
// notification messages from swupdate. Progress messages are forwarded to the GUI thread
// through a channel, notification messages are collected in a list.

use crate::ffi::swupdate as sys;
use std::cell::UnsafeCell;
use std::ffi::{c_char, c_int, CStr};
use std::fs::File;
use std::io::Read;
use std::mem::MaybeUninit;
use std::sync::mpsc::Sender;
use std::sync::{Mutex, OnceLock};

pub use sys::{DONE, DOWNLOAD, FAILURE, IDLE, PROGRESS, RUN, START, SUBPROCESS, SUCCESS};

/// Progress information received from swupdate
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Progress {
    pub status: sys::RECOVERY_STATUS,
    pub source: sys::sourcetype,
    pub nsteps: u32,
    pub cur_step: u32,
    pub cur_percent: u32,
    pub info: String,
}

impl From<&sys::progress_msg> for Progress {
    fn from(msg: &sys::progress_msg) -> Self {
        // The structure is packed: copy the fields before using them.
        let info = msg.info;
        let infolen = (msg.infolen as usize).min(info.len());
        let info: Vec<u8> = info[..infolen].iter().map(|&c| c as u8).collect();

        Progress {
            status: msg.status,
            source: msg.source,
            nsteps: msg.nsteps,
            cur_step: msg.cur_step,
            cur_percent: msg.cur_percent,
            info: String::from_utf8_lossy(&info)
                .trim_end_matches('\0')
                .to_string(),
        }
    }
}

/* Channel to the GUI thread */
static PROGRESS_SENDER: OnceLock<Sender<Progress>> = OnceLock::new();

/* Notification messages received from swupdate during the last update */
static MESSAGES: Mutex<Vec<String>> = Mutex::new(Vec::new());

/* The swu file which is currently sent to swupdate */
static UPDATE_FILE: Mutex<Option<File>> = Mutex::new(None);

fn push_message(message: impl Into<String>) {
    MESSAGES.lock().unwrap().push(message.into());
}

fn send_progress(progress: Progress) {
    if let Some(sender) = PROGRESS_SENDER.get() {
        if sender.send(progress).is_err() {
            log::error!("Not able to send progress message, the GUI is gone");
        }
    }
}

fn send_status(status: sys::RECOVERY_STATUS, info: &str) {
    send_progress(Progress {
        status,
        info: info.into(),
        ..Default::default()
    });
}

fn source_name(source: sys::sourcetype) -> String {
    match source {
        sys::SOURCE_UNKNOWN => "UNKNOWN".into(),
        sys::SOURCE_WEBSERVER => "WEBSERVER".into(),
        sys::SOURCE_SURICATTA => "BACKEND".into(),
        sys::SOURCE_DOWNLOADER => "DOWNLOADER".into(),
        sys::SOURCE_CHUNKS_DOWNLOADER => "CHUNKS DOWNLOADER".into(),
        sys::SOURCE_LOCAL => "LOCAL".into(),
        other => {
            log::warn!("Invalid source: {other}");
            format!("({other})")
        }
    }
}

/// State of the IPC thread
struct IpcConnection {
    progress_fd: c_int,
    notify_fd: c_int,
    waiting_for_update: bool,
}

impl IpcConnection {
    fn connect(&mut self) {
        if self.notify_fd >= 0 {
            unsafe { libc::close(self.notify_fd) };
            self.notify_fd = -1;
        }

        self.progress_fd = unsafe { sys::progress_ipc_connect(true) };
        if self.progress_fd < 0 {
            log::error!("Failed to connect to the swupdate socket.");
            std::thread::sleep(std::time::Duration::from_secs(1));
            return;
        }

        self.notify_fd = unsafe { sys::ipc_notify_connect() };
        if self.notify_fd < 0 {
            let msg = "Failed to connect to the swupdate socket to get notification messages.";
            log::error!("{msg}");
            push_message(msg);
        }
    }

    fn track_update(&mut self, status: sys::RECOVERY_STATUS, source: sys::sourcetype) {
        if self.waiting_for_update && (status == START || status == RUN) {
            log::info!("IPCHandler: Interface: {}", source_name(source));
            self.waiting_for_update = false;
        }

        match status {
            FAILURE | SUCCESS => self.waiting_for_update = true,
            DONE => log::info!("swupdate: DONE."),
            START | IDLE | RUN | DOWNLOAD | SUBPROCESS | PROGRESS => {}
            other => log::warn!("Invalid update status {other} received."),
        }
    }

    fn receive_progress(&mut self) {
        let mut msg = MaybeUninit::<sys::progress_msg>::zeroed();
        let res = unsafe { sys::progress_ipc_receive(&mut self.progress_fd, msg.as_mut_ptr()) };

        if res > 0 {
            let progress = Progress::from(unsafe { msg.assume_init_ref() });
            self.track_update(progress.status, progress.source);
            send_progress(progress);
        }
    }

    fn receive_notification(&mut self) {
        let mut msg = MaybeUninit::<sys::ipc_message>::zeroed();
        let res = unsafe { sys::ipc_notify_receive(&mut self.notify_fd, msg.as_mut_ptr()) };

        if res > 0 {
            let text = unsafe {
                let msg = msg.assume_init_ref();
                CStr::from_ptr(msg.data.notify.msg.as_ptr())
                    .to_string_lossy()
                    .into_owned()
            };
            push_message(text);
        } else {
            let msg = "Failed to read a message from the swupdate socket. \
                       Stopping collection of further messages.";
            log::error!("{msg}");
            push_message(msg);
            if self.notify_fd >= 0 {
                unsafe { libc::close(self.notify_fd) };
                self.notify_fd = -1;
            }
        }
    }

    fn poll(&mut self) {
        let mut fds = [
            libc::pollfd {
                fd: self.progress_fd,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: self.notify_fd,
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        let ready = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, -1) };
        if ready <= 0 {
            return;
        }

        if fds[0].revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0 {
            self.receive_progress();
        }
        if fds[1].revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR) != 0 {
            self.receive_notification();
        }
    }

    fn run(mut self) {
        loop {
            if self.progress_fd < 0 {
                self.connect();
            } else {
                self.poll();
            }
        }
    }
}

/// Starts the thread monitoring swupdate IPC messages. Progress messages are sent to `sender`.
pub fn start_ipc_thread(sender: Sender<Progress>) {
    if PROGRESS_SENDER.set(sender).is_err() {
        log::error!("The swupdate IPC thread is already running");
        return;
    }

    let connection = IpcConnection {
        progress_fd: -1,
        notify_fd: -1,
        waiting_for_update: true,
    };

    if let Err(err) = std::thread::Builder::new()
        .name("swupdate-ipc".into())
        .spawn(move || connection.run())
    {
        log::error!("Failed to create a thread to communicate with the swupdate process: {err}");
        std::process::exit(-1);
    }
}

/* Buffer handed to libswupdate; only used by its single update thread. */
struct ChunkBuffer(UnsafeCell<[u8; 16 * 1024]>);

// SAFETY: libswupdate calls get_next_chunk() from one thread, and consumes the buffer
// before requesting the next chunk.
unsafe impl Sync for ChunkBuffer {}

static CHUNK: ChunkBuffer = ChunkBuffer(UnsafeCell::new([0; 16 * 1024]));

/// Called by libswupdate to read the next chunk of the local update file
unsafe extern "C" fn get_next_chunk(buf: *mut *mut c_char, size: *mut c_int) -> c_int {
    let chunk = unsafe { &mut *CHUNK.0.get() };

    let read = match UPDATE_FILE.lock().unwrap().as_mut() {
        Some(file) => file.read(chunk),
        None => Ok(0),
    };

    let len = read.unwrap_or_else(|err| {
        let msg = "Error reading from the swu file.";
        log::error!("{msg} {err}");
        push_message(msg);
        0
    });

    unsafe {
        *buf = chunk.as_mut_ptr() as *mut c_char;
        *size = len as c_int;
    }
    0
}

/// Called by libswupdate when the update finished
unsafe extern "C" fn end_of_update(status: sys::RECOVERY_STATUS) -> c_int {
    if status == SUCCESS {
        log::info!("EndOfUpdate: SWUpdate was successful!");
    } else {
        log::info!("EndOfUpdate: SWUpdate failed!");
    }

    UPDATE_FILE.lock().unwrap().take();
    0
}

/// Start an update with the given local file
pub fn start_local_update(filename: &str, dry_run: bool) {
    MESSAGES.lock().unwrap().clear();

    let file = match File::open(filename) {
        Ok(file) => file,
        Err(err) => {
            log::warn!("Local-update: Unable to open file {filename}: {err}");
            send_status(FAILURE, "Unable to open file");
            return;
        }
    };
    *UPDATE_FILE.lock().unwrap() = Some(file);

    let rc = unsafe {
        let mut request = MaybeUninit::<sys::swupdate_request>::zeroed();
        sys::swupdate_prepare_req(request.as_mut_ptr());
        let mut request = request.assume_init();

        request.dry_run = if dry_run {
            sys::RUN_DRYRUN
        } else {
            sys::RUN_DEFAULT
        };

        sys::swupdate_async_start(
            Some(get_next_chunk),
            None,
            Some(end_of_update),
            &mut request as *mut _ as *mut _,
            std::mem::size_of::<sys::swupdate_request>() as _,
        )
    };

    if rc < 0 {
        UPDATE_FILE.lock().unwrap().take();

        let msg = "The swupdate_async_start() function failed. Check the swupdate service state.";
        log::error!("{msg} The return code: {rc}");
        send_status(FAILURE, msg);
    }
}

/// All notification messages received from swupdate during the last update
pub fn messages() -> Vec<String> {
    MESSAGES.lock().unwrap().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_from_packed_message() {
        let mut msg: sys::progress_msg = unsafe { std::mem::zeroed() };
        msg.status = RUN;
        msg.nsteps = 4;
        msg.cur_step = 2;
        msg.cur_percent = 55;
        msg.source = sys::SOURCE_LOCAL;
        let text = b"hello";
        let mut info = msg.info;
        for (dst, src) in info.iter_mut().zip(text) {
            *dst = *src as c_char;
        }
        msg.info = info;
        msg.infolen = text.len() as u32;

        let progress = Progress::from(&msg);

        assert_eq!(progress.status, RUN);
        assert_eq!(
            (progress.cur_step, progress.nsteps, progress.cur_percent),
            (2, 4, 55)
        );
        assert_eq!(progress.source, sys::SOURCE_LOCAL);
        assert_eq!(progress.info, "hello");
    }

    #[test]
    fn source_names() {
        assert_eq!(source_name(sys::SOURCE_SURICATTA), "BACKEND");
        assert_eq!(source_name(42), "(42)");
    }
}
