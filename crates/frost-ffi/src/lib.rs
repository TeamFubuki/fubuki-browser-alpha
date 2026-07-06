use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender};
use frost_core::BrowserCore;
use frost_protocol::{EventEnvelope, ProtocolRequest, ProtocolResponse};

pub struct FrostEngineHandle {
    request_tx: Sender<ProtocolRequest>,
    response_rx: Receiver<ProtocolResponse>,
    event_rx: Receiver<EventEnvelope>,
    join_handle: Option<JoinHandle<()>>,
}

/// Creates a new FrostEngine instance and returns a handle to it.
///
/// The caller is responsible for freeing the handle with `frost_engine_free`.
#[unsafe(no_mangle)]
pub extern "C" fn frost_engine_new() -> *mut FrostEngineHandle {
    let (request_tx, request_rx) = crossbeam_channel::unbounded();
    let (response_tx, response_rx) = crossbeam_channel::unbounded();
    let (event_tx, event_rx) = crossbeam_channel::unbounded();

    let mut core = BrowserCore::new();
    core.set_event_sender(event_tx);
    let join_handle = std::thread::spawn(move || core.run(request_rx, response_tx));

    Box::into_raw(Box::new(FrostEngineHandle {
        request_tx,
        response_rx,
        event_rx,
        join_handle: Some(join_handle),
    }))
}

/// # Safety
///
/// - `handle` must be a valid pointer obtained from `frost_engine_new`.
/// - After calling this function, the handle must not be used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_free(handle: *mut FrostEngineHandle) {
    if handle.is_null() {
        return;
    }
    unsafe {
        let mut handle = Box::from_raw(handle);
        drop(handle.request_tx);
        if let Some(join_handle) = handle.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

/// # Safety
///
/// - `handle` must be a valid pointer obtained from `frost_engine_new`.
/// - `request_json` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_process_json(
    handle: *mut FrostEngineHandle,
    request_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || request_json.is_null() {
        return into_c_string(error_response(None, "handle or request was null"));
    }

    unsafe {
        let request_text = match CStr::from_ptr(request_json).to_str() {
            Ok(text) => text,
            Err(error) => return into_c_string(error_response(None, error.to_string())),
        };
        let request = match serde_json::from_str::<ProtocolRequest>(request_text) {
            Ok(request) => request,
            Err(error) => return into_c_string(error_response(None, error.to_string())),
        };

        let id = request.id.clone();
        let handle = &*handle;
        if let Err(error) = handle.request_tx.send(request) {
            return into_c_string(error_response(id, error.to_string()));
        }
        match handle.response_rx.recv() {
            Ok(response) => into_c_string(serde_json::to_string(&response).unwrap_or_else(|e| {
                serde_json::to_string(&ProtocolResponse::error(id, e.to_string()))
                    .unwrap_or_default()
            })),
            Err(error) => into_c_string(error_response(id, error.to_string())),
        }
    }
}

/// # Safety
///
/// - `handle` must be a valid pointer obtained from `frost_engine_new`.
/// - Returns a null pointer if no event is available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_poll_event_json(
    handle: *mut FrostEngineHandle,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let handle = &*handle;
        match handle.event_rx.try_recv() {
            Ok(event) => into_c_string(serde_json::to_string(&event).unwrap_or_default()),
            Err(_) => ptr::null_mut(),
        }
    }
}

/// # Safety
///
/// - `value` must be a valid pointer obtained from one of the `frost_engine_*` functions.
/// - After calling this function, the pointer must not be used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_string_free(value: *mut c_char) {
    if !value.is_null() {
        unsafe {
            let _ = CString::from_raw(value);
        }
    }
}

fn error_response(id: Option<String>, message: impl Into<String>) -> String {
    serde_json::to_string(&ProtocolResponse::error(id, message.into())).unwrap_or_default()
}

fn into_c_string(value: String) -> *mut c_char {
    CString::new(value).unwrap_or_default().into_raw()
}
