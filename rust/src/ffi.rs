//! The C ABI consumed by the Qt/KDE side of the widget.

use std::ffi::{CStr, CString, c_char};

use crate::engine::{self, UiEvent};
use crate::json::{Json, Obj};

pub const OP_ADD: u32 = 1;
pub const OP_REMOVE: u32 = 2;
pub const OP_SET_ENABLED: u32 = 3;
pub const OP_MARK_READ: u32 = 4;
pub const OP_MARK_ALL_READ: u32 = 5;

/// Notifications below this priority stay in the unread counter only.
const DEFAULT_PRIORITY: u8 = 3;

/// Starts the engine and every subscription stream. Returns non-zero on success.
#[unsafe(no_mangle)]
pub extern "C" fn ntfy_start() -> i32 {
    let engine = engine::engine();
    engine.start();
    i32::from(engine.is_running())
}

/// Stops all streams. Safe to call when not running.
#[unsafe(no_mangle)]
pub extern "C" fn ntfy_stop() {
    engine::engine().stop();
}

#[unsafe(no_mangle)]
pub extern "C" fn ntfy_is_running() -> bool {
    engine::engine().is_running()
}

/// Runs a command and returns a JSON result the caller frees with
/// `ntfy_string_free`. Any string argument may be null, meaning an empty string.
///
/// # Safety
///
/// Every pointer must be null or a valid NUL-terminated C string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ntfy_command(
    op: u32,
    a: *const c_char,
    b: *const c_char,
    c: *const c_char,
) -> *mut c_char {
    let engine = engine::engine();
    let a = unsafe { opt_str(a) };
    let b = unsafe { opt_str(b) };
    let c = unsafe { opt_str(c) };

    let result = match op {
        // a = server, b = topic, c = access token
        OP_ADD => engine.add(&a, &b, &c, DEFAULT_PRIORITY),
        OP_REMOVE => engine.remove(&a),
        OP_SET_ENABLED => engine.set_enabled(&a, b == "1"),
        OP_MARK_READ => engine.mark_read(&a),
        OP_MARK_ALL_READ => engine.mark_all_read(),
        _ => Err(format!("unknown command {op}")),
    };

    let json = match result {
        Ok(()) => Obj::new().set("ok", Json::Bool(true)).build(),
        Err(e) => Obj::new()
            .set("ok", Json::Bool(false))
            .set("error", Json::str(e))
            .build(),
    };
    into_c(json.to_text())
}

/// Frees a string returned by `ntfy_command`.
///
/// # Safety
///
/// The pointer must be null or have come from `ntfy_command`, and must not be used
/// again afterwards.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ntfy_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(unsafe { CString::from_raw(s) });
    }
}

/// Registers a listener. The callback receives `(kind, json)` and must not block.
/// Returns an opaque handle for `ntfy_remove_sink`.
///
/// # Safety
///
/// `sink` must stay valid for as long as the handle is registered.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ntfy_add_sink(sink: extern "C" fn(u32, *const c_char, usize)) -> usize {
    engine::engine().add_sink(Box::new(move |event: UiEvent| {
        if let Ok(c) = CString::new(event.json) {
            sink(event.kind, c.as_ptr(), c.as_bytes().len());
        }
    }))
}

/// # Safety
///
/// The handle must come from `ntfy_add_sink` and must not be used twice.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn ntfy_remove_sink(handle: usize) {
    engine::engine().remove_sink(handle);
}

fn into_c(text: String) -> *mut c_char {
    CString::new(text)
        .unwrap_or_else(|_| CString::new("{}").expect("literal is NUL free"))
        .into_raw()
}

unsafe fn opt_str(s: *const c_char) -> String {
    if s.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(s) }.to_string_lossy().into_owned()
    }
}
