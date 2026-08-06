use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::thread::JoinHandle;

use crossbeam_channel::{Receiver, Sender, select};
use frost_core::{BrowserCore, ExternalPolicy, HostCommandAdapter};
use frost_engine_api::EngineAdapter;
use frost_protocol::{
    EventEnvelope, HostCommandEnvelope, HostCommandResultEnvelope, HostEventEnvelope,
    ProtocolRequest, ProtocolResponse,
};
use frost_store::{
    BookmarkRepository, ClearRepository, DownloadRepository, HistoryRepository, LogRepository,
    PermissionRepository, SessionRepository, SettingsRepository, SqliteStore,
};

pub struct FrostEngineHandle {
    request_tx: Sender<ProtocolRequest>,
    response_rx: Receiver<ProtocolResponse>,
    event_rx: Receiver<EventEnvelope>,
    host_command_rx: Receiver<HostCommandEnvelope>,
    host_event_tx: Sender<HostEventEnvelope>,
    host_result_tx: Sender<HostCommandResultEnvelope>,
    // Channel for external audit/rate-limit events emitted by the FFI layer.
    external_event_tx: Sender<frost_protocol::ExternalEventEnvelope>,
    external_policy: std::sync::Mutex<ExternalPolicy>,
    // Serializes the request_tx send + response_rx recv pair so that the JSON
    // processing path and the external routing path (which share the same
    // request/response channels) never read another caller's response.
    request_response_lock: std::sync::Mutex<()>,
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
    let (external_event_tx, _external_event_rx) = crossbeam_channel::unbounded();

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
        external_event_tx,
        external_policy: std::sync::Mutex::new(ExternalPolicy::new()),
        request_response_lock: std::sync::Mutex::new(()),
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
        + LogRepository
        + SessionRepository
        + ClearRepository
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
        // Hold the lock across send + recv so the shared response channel is
        // never read by the wrong caller (see route_external_to_core).
        let _guard = handle
            .request_response_lock
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Err(error) = handle.request_tx.send(request) {
            return into_c_string(error_response(id, error.to_string()));
        }
        match handle.response_rx.recv() {
            Ok(response) => match serde_json::to_string(&response) {
                Ok(json) => into_c_string(json),
                Err(e) => {
                    eprintln!("[frost-ffi] Failed to serialize response: {e}");
                    into_c_string(error_response(id, "response serialization failed"))
                }
            },
            Err(error) => {
                eprintln!("[frost-ffi] Response channel closed: {error}");
                into_c_string(error_response(id, error.to_string()))
            }
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

/// Grants external capabilities to a caller origin.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_engine_new`.
/// - `origin` and `capabilities_json` must be valid null-terminated UTF-8 strings.
/// - `capabilities_json` must be a JSON array of capability strings
///   (e.g. `["read_state","tab_control"]`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_grant_external(
    handle: *mut FrostEngineHandle,
    origin: *const c_char,
    capabilities_json: *const c_char,
) -> bool {
    if handle.is_null() || origin.is_null() || capabilities_json.is_null() {
        return false;
    }
    let origin = match unsafe { CStr::from_ptr(origin) }.to_str() {
        Ok(o) => o.to_owned(),
        Err(_) => return false,
    };
    let json = match unsafe { CStr::from_ptr(capabilities_json) }.to_str() {
        Ok(j) => j,
        Err(_) => return false,
    };
    let capabilities: Vec<frost_protocol::ExternalCapability> = match serde_json::from_str(json) {
        Ok(caps) => caps,
        Err(_) => return false,
    };
    let handle = unsafe { &*handle };
    if let Ok(mut policy) = handle.external_policy.lock() {
        policy.grant(&origin, capabilities);
        true
    } else {
        false
    }
}

/// Processes an external (MCP) command JSON string and returns a JSON response.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_engine_new`.
/// - `command_json` must be a valid null-terminated UTF-8 string representing an
///   `ExternalCommandEnvelope`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_process_external_json(
    handle: *mut FrostEngineHandle,
    command_json: *const c_char,
) -> *mut c_char {
    if handle.is_null() || command_json.is_null() {
        return into_c_string(
            serde_json::json!({ "allowed": false, "error": "null handle or command" }).to_string(),
        );
    }
    let text = match unsafe { CStr::from_ptr(command_json) }.to_str() {
        Ok(t) => t,
        Err(_) => {
            return into_c_string(
                serde_json::json!({ "allowed": false, "error": "invalid utf-8" }).to_string(),
            );
        }
    };
    let envelope: frost_protocol::ExternalCommandEnvelope = match serde_json::from_str(text) {
        Ok(e) => e,
        Err(err) => {
            return into_c_string(
                serde_json::json!({ "allowed": false, "error": err.to_string() }).to_string(),
            );
        }
    };
    let handle = unsafe { &*handle };
    let mut policy = match handle.external_policy.lock() {
        Ok(p) => p,
        Err(_) => {
            return into_c_string(
                serde_json::json!({ "allowed": false, "error": "policy poisoned" }).to_string(),
            );
        }
    };
    // Route the command through the live engine's request channel and read the
    // matching response. Capability and rate-limit checks are re-run here
    // (gated by the caller `origin`, never by the correlation `id`) because the
    // external path cannot borrow the core owned by the worker thread. The
    // request/response pair is serialized via `request_response_lock`.
    let response = route_external_to_core(handle, envelope, &mut policy);
    into_c_string(response)
}

/// Routes an external command to the engine request channel.
///
/// This keeps a single command path: external clients go through the same
/// `ProtocolRequest` pipeline as the UI, so no host/CEF object is ever touched
/// directly by automation.
fn route_external_to_core(
    handle: &FrostEngineHandle,
    envelope: frost_protocol::ExternalCommandEnvelope,
    policy: &mut ExternalPolicy,
) -> String {
    use frost_protocol::{ExternalCapability, ExternalCommand, ProtocolRequest, Request, Response};

    let capability = match &envelope.command {
        ExternalCommand::StateRead => ExternalCapability::ReadState,
        ExternalCommand::TabCreate { .. } => ExternalCapability::TabControl,
        ExternalCommand::TabClose { .. } => ExternalCapability::TabControl,
        ExternalCommand::NavigationOpen { .. } => ExternalCapability::Navigation,
        ExternalCommand::BookmarkSave { .. } => ExternalCapability::Bookmarks,
        ExternalCommand::HistoryClear { .. } => ExternalCapability::History,
        ExternalCommand::DownloadRemove { .. } => ExternalCapability::Downloads,
        ExternalCommand::DebugOpenDevTools { .. } => ExternalCapability::Debug,
    };

    // Gate by the caller origin, never by the correlation id, and never trust
    // the capability declared inside the command envelope.
    if !policy.is_granted(&envelope.origin, &capability) {
        // Emit audit event for denied capability.
        let _ = handle
            .external_event_tx
            .send(frost_protocol::ExternalEventEnvelope::new(
                frost_protocol::ExternalEvent::Audit {
                    command_id: envelope.id.clone(),
                    capability,
                    allowed: false,
                    reason: Some("capability not granted".into()),
                },
            ));
        return serde_json::json!({ "allowed": false, "error": "capability not granted" })
            .to_string();
    }
    if !policy.check_rate(&envelope.origin) {
        // Emit rate-limit and audit events.
        let _ = handle
            .external_event_tx
            .send(frost_protocol::ExternalEventEnvelope::new(
                frost_protocol::ExternalEvent::RateLimited {
                    command_id: envelope.id.clone(),
                    retry_after_ms: 60_000,
                },
            ));
        let _ = handle
            .external_event_tx
            .send(frost_protocol::ExternalEventEnvelope::new(
                frost_protocol::ExternalEvent::Audit {
                    command_id: envelope.id.clone(),
                    capability,
                    allowed: false,
                    reason: Some("rate limited".into()),
                },
            ));
        return serde_json::json!({ "allowed": false, "error": "rate limited" }).to_string();
    }

    let request: Option<ProtocolRequest> = match &envelope.command {
        ExternalCommand::StateRead => Some(ProtocolRequest::new(Request::AppSnapshot)),
        ExternalCommand::TabCreate { url, active } => {
            Some(ProtocolRequest::new(Request::TabsCreate {
                url: url.clone(),
                active: *active,
                window_id: None,
            }))
        }
        ExternalCommand::TabClose { tab_id } => Some(ProtocolRequest::new(Request::TabsClose {
            tab_id: tab_id.clone(),
        })),
        ExternalCommand::NavigationOpen { tab_id, input } => {
            Some(ProtocolRequest::new(Request::TabsNavigate {
                tab_id: tab_id.clone(),
                input: input.clone(),
            }))
        }
        ExternalCommand::BookmarkSave {
            title,
            url,
            favicon_url,
        } => Some(ProtocolRequest::new(Request::BookmarksSave {
            title: title.clone(),
            url: url.clone(),
            favicon_url: favicon_url.clone(),
        })),
        ExternalCommand::HistoryClear { range } => {
            Some(ProtocolRequest::new(Request::HistoryClearRange {
                range: range.clone(),
            }))
        }
        ExternalCommand::DownloadRemove { url, path } => {
            Some(ProtocolRequest::new(Request::DownloadsRemove {
                url: url.clone(),
                path: path.clone(),
            }))
        }
        ExternalCommand::DebugOpenDevTools { .. } => None,
    };

    match request {
        Some(req) => {
            // Serialize send + recv so the shared response channel is never read
            // by the wrong caller (see frost_engine_process_json).
            let _guard = handle
                .request_response_lock
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if handle.request_tx.send(req).is_err() {
                // Emit audit for engine unavailable.
                let _ = handle
                    .external_event_tx
                    .send(frost_protocol::ExternalEventEnvelope::new(
                        frost_protocol::ExternalEvent::Audit {
                            command_id: envelope.id.clone(),
                            capability,
                            allowed: false,
                            reason: Some("engine unavailable".into()),
                        },
                    ));
                return serde_json::json!({ "allowed": false, "error": "engine unavailable" })
                    .to_string();
            }
            match handle.response_rx.recv() {
                Ok(resp) => {
                    let mut obj = serde_json::Map::new();
                    obj.insert("allowed".into(), serde_json::json!(resp.ok));
                    obj.insert("ok".into(), serde_json::json!(resp.ok));
                    // Emit audit for successful or failed routing.
                    let _ =
                        handle
                            .external_event_tx
                            .send(frost_protocol::ExternalEventEnvelope::new(
                                frost_protocol::ExternalEvent::Audit {
                                    command_id: envelope.id.clone(),
                                    capability,
                                    allowed: resp.ok,
                                    reason: if resp.ok {
                                        None
                                    } else {
                                        Some("request failed".into())
                                    },
                                },
                            ));
                    // Surface the snapshot body for state reads instead of just
                    // an `ok` flag.
                    if let Response::AppSnapshot(state) = &resp.response {
                        obj.insert(
                            "result".into(),
                            serde_json::to_value(state).unwrap_or(serde_json::Value::Null),
                        );
                    }
                    serde_json::Value::Object(obj).to_string()
                }
                Err(_) => {
                    // Emit audit for no response.
                    let _ =
                        handle
                            .external_event_tx
                            .send(frost_protocol::ExternalEventEnvelope::new(
                                frost_protocol::ExternalEvent::Audit {
                                    command_id: envelope.id.clone(),
                                    capability,
                                    allowed: false,
                                    reason: Some("no response".into()),
                                },
                            ));
                    serde_json::json!({ "allowed": false, "error": "no response" }).to_string()
                }
            }
        }
        None => {
            // DebugOpenDevTools (and any unsupported command) is a host/CEF
            // concern the engine must not perform; never report success.
            // Emit audit for unsupported command.
            let _ = handle
                .external_event_tx
                .send(frost_protocol::ExternalEventEnvelope::new(
                    frost_protocol::ExternalEvent::Audit {
                        command_id: envelope.id.clone(),
                        capability,
                        allowed: false,
                        reason: Some("command not supported via external router".into()),
                    },
                ));
            serde_json::json!({ "allowed": false, "error": "command not supported via external router" })
                .to_string()
        }
    }
}

/// Opaque handle to a standalone SQLite store owned by the engine.
///
/// The native host no longer owns browser data; it delegates persistence to
/// FrostEngine's `frost-store` through this handle.
pub struct FrostStoreHandle {
    store: SqliteStore,
}

/// Opens (or creates) an engine-owned SQLite store at `path`.
///
/// # Safety
/// - `path` must be a valid null-terminated UTF-8 string, or null.
/// - Returns a valid non-null handle on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_open(path: *const c_char) -> *mut FrostStoreHandle {
    let path = if path.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(path) }
            .to_str()
            .unwrap_or_default()
            .to_owned()
    };
    let store = if path.is_empty() {
        SqliteStore::in_memory()
    } else {
        SqliteStore::open(path)
    };
    match store {
        Ok(store) => Box::into_raw(Box::new(FrostStoreHandle { store })),
        Err(_) => ptr::null_mut(),
    }
}

/// Frees a store handle previously returned by `frost_store_open`.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_store_open`.
/// - After calling this function, the handle must not be used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_free(handle: *mut FrostStoreHandle) {
    if handle.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(handle));
    }
}

/// Reads a setting value as JSON string, or `null` if absent.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_store_open`.
/// - `key` must be a valid null-terminated UTF-8 string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_get_setting(
    handle: *mut FrostStoreHandle,
    key: *const c_char,
) -> *mut c_char {
    if handle.is_null() || key.is_null() {
        return ptr::null_mut();
    }
    let key = match unsafe { CStr::from_ptr(key) }.to_str() {
        Ok(k) => k,
        Err(_) => return ptr::null_mut(),
    };
    let store = unsafe { &*handle };
    match store.store.get_setting(key) {
        Ok(Some(value)) => into_c_string(value),
        _ => ptr::null_mut(),
    }
}

/// Writes a setting value. Returns true on success.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_store_open`.
/// - `key` and `value` must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_set_setting(
    handle: *mut FrostStoreHandle,
    key: *const c_char,
    value: *const c_char,
) -> bool {
    if handle.is_null() || key.is_null() || value.is_null() {
        return false;
    }
    let key = match unsafe { CStr::from_ptr(key) }.to_str() {
        Ok(k) => k,
        Err(_) => return false,
    };
    let value = match unsafe { CStr::from_ptr(value) }.to_str() {
        Ok(v) => v,
        Err(_) => return false,
    };
    let store = unsafe { &*handle };
    store.store.set_setting(key, value).is_ok()
}

/// Returns all settings as a JSON object string.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_store_open`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_get_all_settings(
    handle: *mut FrostStoreHandle,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    let store = unsafe { &*handle };
    let mut map = serde_json::Map::new();
    for key in [
        "homepage",
        "searchEngine",
        "customSearchUrl",
        "startupBehavior",
        "sessionJson",
        "downloadDirectory",
        "theme",
        "appearance",
        "toolbarDensity",
        "sidebarVisible",
        "sidebarWidth",
        "defaultBookmarkDisplay",
        "openBookmarkIn",
        "showBookmarkFavicons",
        "newTabPage",
        "homeUrl",
        "askBeforeDownload",
        "defaultZoomLevel",
        "closeWindowWithLastTab",
        "privateSearchEngine",
        "language",
        "newTabBackgroundMode",
        "newTabBackgroundColor",
        "newTabBackgroundUrl",
    ] {
        if let Ok(Some(value)) = store.store.get_setting(key) {
            map.insert(key.to_string(), serde_json::Value::String(value));
        }
    }
    into_c_string(serde_json::Value::Object(map).to_string())
}

/// Appends a log entry. Returns true on success.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_store_open`.
/// - `level` and `message` must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_add_log(
    handle: *mut FrostStoreHandle,
    level: *const c_char,
    message: *const c_char,
) -> bool {
    if handle.is_null() || level.is_null() || message.is_null() {
        return false;
    }
    let level = match unsafe { CStr::from_ptr(level) }.to_str() {
        Ok(l) => l,
        Err(_) => return false,
    };
    let message = match unsafe { CStr::from_ptr(message) }.to_str() {
        Ok(m) => m,
        Err(_) => return false,
    };
    let store = unsafe { &*handle };
    store.store.add_log(level, message).is_ok()
}

/// Returns recent logs as a JSON array string.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_store_open`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_get_logs(
    handle: *mut FrostStoreHandle,
    limit: usize,
) -> *mut c_char {
    if handle.is_null() {
        return ptr::null_mut();
    }
    let store = unsafe { &*handle };
    match store.store.list_logs(limit) {
        Ok(logs) => match serde_json::to_string(&logs) {
            Ok(json) => into_c_string(json),
            Err(_) => ptr::null_mut(),
        },
        Err(_) => ptr::null_mut(),
    }
}

/// Clears all logs. Returns true on success.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_store_open`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_clear_logs(handle: *mut FrostStoreHandle) -> bool {
    if handle.is_null() {
        return false;
    }
    let store = unsafe { &*handle };
    store.store.clear_logs().is_ok()
}

/// Adds a history entry. Returns true on success.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_store_open`.
/// - `title`, `url`, `favicon_url` must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_add_history(
    handle: *mut FrostStoreHandle,
    title: *const c_char,
    url: *const c_char,
    favicon_url: *const c_char,
) -> bool {
    if handle.is_null() || title.is_null() || url.is_null() || favicon_url.is_null() {
        return false;
    }
    let title = match unsafe { CStr::from_ptr(title) }.to_str() {
        Ok(t) => t,
        Err(_) => return false,
    };
    let url = match unsafe { CStr::from_ptr(url) }.to_str() {
        Ok(u) => u,
        Err(_) => return false,
    };
    let favicon_url = match unsafe { CStr::from_ptr(favicon_url) }.to_str() {
        Ok(f) => f,
        Err(_) => return false,
    };
    let store = unsafe { &*handle };
    store.store.add_history(title, url, favicon_url).is_ok()
}

/// Inserts or updates a download record. Returns true on success.
///
/// # Safety
/// - `handle` must be a valid pointer obtained from `frost_store_open`.
/// - `url`, `path`, `state` must be valid null-terminated UTF-8 strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_upsert_download(
    handle: *mut FrostStoreHandle,
    url: *const c_char,
    path: *const c_char,
    state: *const c_char,
    percent: i64,
) -> bool {
    if handle.is_null() || url.is_null() || path.is_null() || state.is_null() {
        return false;
    }
    let url = match unsafe { CStr::from_ptr(url) }.to_str() {
        Ok(u) => u,
        Err(_) => return false,
    };
    let path = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let state = match unsafe { CStr::from_ptr(state) }.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };
    let store = unsafe { &*handle };
    store
        .store
        .upsert_download(url, path, state, percent)
        .is_ok()
}

/// Frees a string previously returned by a `frost_store_*` function.
///
/// # Safety
/// - `value` must be a valid pointer obtained from a `frost_store_*` function.
/// - After calling this function, the pointer must not be used again.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_store_string_free(value: *mut c_char) {
    unsafe {
        frost_engine_string_free(value);
    }
}
