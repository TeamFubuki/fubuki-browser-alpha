use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender, select};
use frost_core::{BrowserCore, HostCommandAdapter};
use frost_engine_api::EngineAdapter;
use frost_protocol::{
    EventEnvelope, HostCommandEnvelope, HostCommandResultEnvelope, HostEventEnvelope,
    ProtocolRequest, ProtocolResponse,
};
use frost_store::{
    BookmarkRepository, DownloadRepository, HistoryRepository, PermissionRepository,
    SettingsRepository,
};

pub struct FrostEngineHandle {
    request_tx: Sender<ProtocolRequest>,
    response_rx: Receiver<ProtocolResponse>,
    event_rx: Receiver<EventEnvelope>,
    host_command_rx: Receiver<HostCommandEnvelope>,
    host_event_tx: Sender<HostEventEnvelope>,
    host_result_tx: Sender<HostCommandResultEnvelope>,
    join_handle: Option<JoinHandle<()>>,
}

/// Creates a new FrostEngine instance and returns a handle to it.
///
/// The caller is responsible for freeing the handle with `frost_engine_free`.
///
/// # Safety
///
/// Returns a valid non-null handle on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_new() -> *mut FrostEngineHandle {
    unsafe { frost_engine_new_with_store(ptr::null()) }
}

/// Creates a new FrostEngine instance backed by a SQLite store at `path`.
///
/// If `path` is null or empty, falls back to an in-memory store.
///
/// # Safety
///
/// - `path` must be a valid null-terminated UTF-8 string, or null.
/// - Returns a valid non-null handle on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_new_with_store(
    path: *const c_char,
) -> *mut FrostEngineHandle {
    let (request_tx, request_rx) = crossbeam_channel::unbounded();
    let (response_tx, response_rx) = crossbeam_channel::unbounded();
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let (host_command_tx, host_command_rx) = crossbeam_channel::unbounded();
    let (host_event_tx, host_event_rx) = crossbeam_channel::unbounded();
    let (host_result_tx, host_result_rx) = crossbeam_channel::unbounded();

    let join_handle = if path.is_null() {
        spawn_core(
            BrowserCore::with_adapter_and_settings(
                HostCommandAdapter::new(host_command_tx),
                frost_core::InMemoryStore::default(),
            ),
            event_tx.clone(),
            request_rx.clone(),
            response_tx.clone(),
            host_event_rx,
            host_result_rx,
        )
    } else {
        let path = unsafe { CStr::from_ptr(path) }
            .to_str()
            .unwrap_or_default()
            .to_owned();
        match frost_store::SqliteStore::open(path) {
            Ok(store) => spawn_core(
                BrowserCore::with_adapter_and_settings(
                    HostCommandAdapter::new(host_command_tx.clone()),
                    store,
                ),
                event_tx.clone(),
                request_rx.clone(),
                response_tx.clone(),
                host_event_rx.clone(),
                host_result_rx.clone(),
            ),
            Err(_) => spawn_core(
                BrowserCore::with_adapter_and_settings(
                    HostCommandAdapter::new(host_command_tx.clone()),
                    frost_core::InMemoryStore::default(),
                ),
                event_tx.clone(),
                request_rx.clone(),
                response_tx.clone(),
                host_event_rx.clone(),
                host_result_rx.clone(),
            ),
        }
    };

    Box::into_raw(Box::new(FrostEngineHandle {
        request_tx,
        response_rx,
        event_rx,
        host_command_rx,
        host_event_tx,
        host_result_tx,
        join_handle: Some(join_handle),
    }))
}

fn spawn_core<A, S>(
    mut core: BrowserCore<A, S>,
    event_tx: Sender<EventEnvelope>,
    request_rx: Receiver<ProtocolRequest>,
    response_tx: Sender<ProtocolResponse>,
    host_event_rx: Receiver<HostEventEnvelope>,
    host_result_rx: Receiver<HostCommandResultEnvelope>,
) -> JoinHandle<()>
where
    A: EngineAdapter + Send + 'static,
    S: SettingsRepository
        + BookmarkRepository
        + HistoryRepository
        + DownloadRepository
        + PermissionRepository
        + Send
        + 'static,
{
    core.set_event_sender(event_tx);
    std::thread::spawn(move || {
        loop {
            select! {
                recv(request_rx) -> message => {
                    let Ok(request) = message else {
                        break;
                    };
                    let response = core.process(request);
                    if response_tx.send(response).is_err() {
                        break;
                    }
                }
                recv(host_event_rx) -> message => {
                    if let Ok(event) = message {
                        let _ = core.process_host_event(event);
                    }
                }
                recv(host_result_rx) -> message => {
                    if let Ok(result) = message
                        && let Err(e) = core.process_host_command_result(result)
                    {
                        eprintln!("[frost-engine] host command failed: {e}");
                    }
                }
            }
        }
    })
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
/// - `handle` must be a valid pointer obtained from `frost_engine_new`.
/// - Returns a null pointer if no host command is available.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_poll_host_command_json(
    handle: *mut FrostEngineHandle,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let handle = &*handle;
        match handle.host_command_rx.try_recv() {
            Ok(command) => into_c_string(serde_json::to_string(&command).unwrap_or_default()),
            Err(_) => ptr::null_mut(),
        }
    }
}

/// # Safety
///
/// - `handle` must be a valid pointer obtained from `frost_engine_new`.
/// - `event_json` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_push_host_event_json(
    handle: *mut FrostEngineHandle,
    event_json: *const c_char,
) -> bool {
    if handle.is_null() || event_json.is_null() {
        return false;
    }

    unsafe {
        let event_text = match CStr::from_ptr(event_json).to_str() {
            Ok(text) => text,
            Err(_) => return false,
        };
        let event = match serde_json::from_str::<HostEventEnvelope>(event_text) {
            Ok(event) => event,
            Err(_) => return false,
        };
        (&*handle).host_event_tx.send(event).is_ok()
    }
}

/// # Safety
///
/// - `handle` must be a valid pointer obtained from `frost_engine_new`.
/// - `result_json` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_push_host_command_result_json(
    handle: *mut FrostEngineHandle,
    result_json: *const c_char,
) -> bool {
    if handle.is_null() || result_json.is_null() {
        return false;
    }

    unsafe {
        let result_text = match CStr::from_ptr(result_json).to_str() {
            Ok(text) => text,
            Err(_) => return false,
        };
        let result = match serde_json::from_str::<HostCommandResultEnvelope>(result_text) {
            Ok(result) => result,
            Err(_) => return false,
        };
        (&*handle).host_result_tx.send(result).is_ok()
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
