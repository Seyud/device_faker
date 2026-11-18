// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

use std::sync::Mutex;

use libc::{c_int, c_void};

#[derive(Clone, Copy)]
struct AtexitEntry {
    func: AtexitFn,
    arg: *mut c_void,
}

// Raw pointers are neither Send nor Sync by default; we only move entries across threads
// via the Mutex, so we declare Send to satisfy the static requirement.
unsafe impl Send for AtexitEntry {}

type AtexitFn = unsafe extern "C" fn(*mut c_void);

// Keep registrations in insertion order so we can invoke destructors in reverse.
static ATEXIT_STATE: Mutex<Vec<AtexitEntry>> = Mutex::new(Vec::new());

#[unsafe(no_mangle)]
pub extern "C" fn __cxa_atexit(
    func: Option<AtexitFn>,
    arg: *mut c_void,
    _dso: *mut c_void,
) -> c_int {
    let Some(func) = func else {
        return -1;
    };

    let mut state = ATEXIT_STATE.lock().unwrap();
    state.push(AtexitEntry { func, arg });
    0
}

#[unsafe(no_mangle)]
pub extern "C" fn __cxa_finalize(_dso: *mut c_void) {
    let mut state = ATEXIT_STATE.lock().unwrap();

    loop {
        let entries: Vec<AtexitEntry> = state.drain(..).collect();
        if entries.is_empty() {
            break;
        }
        drop(state);
        for entry in entries.into_iter().rev() {
            unsafe { (entry.func)(entry.arg) };
        }
        state = ATEXIT_STATE.lock().unwrap();
    }
}
