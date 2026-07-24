use std::cell::RefCell;
use std::ffi::{CStr, CString, c_char};
use std::ptr;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, select};
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
    external_policy: Mutex<ExternalPolicy>,
    // Serializes the request_tx send + response_rx recv pair so that the JSON
    // processing path and the external routing path (which share the same
    // request/response channels) never read another caller's response.
    request_response_lock: Mutex<()>,
    shutdown_tx: Sender<()>,
    worker_done_rx: Receiver<()>,
    state: Arc<AtomicU8>,
    next_request_id: AtomicU64,
    join_handle: Option<JoinHandle<()>>,
}

const ENGINE_RUNNING: u8 = 0;
const ENGINE_SHUTTING_DOWN: u8 = 1;
const ENGINE_STOPPED: u8 = 2;
#[cfg(not(test))]
const REQUEST_TIMEOUT: Duration = Duration::from_millis(4_500);
#[cfg(test)]
const REQUEST_TIMEOUT: Duration = Duration::from_millis(100);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_millis(1_500);

#[derive(Clone)]
struct FfiError {
    code: &'static str,
    message: String,
}

impl FfiError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn json(&self) -> serde_json::Value {
        serde_json::json!({ "code": self.code, "message": self.message })
    }
}

thread_local! {
    static LAST_ERROR: RefCell<Option<FfiError>> = const { RefCell::new(None) };
}

fn set_last_error(error: FfiError) {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = Some(error));
}

fn clear_last_error() {
    LAST_ERROR.with(|slot| *slot.borrow_mut() = None);
}

fn init_failed(code: &'static str, message: impl Into<String>) -> *mut FrostEngineHandle {
    set_last_error(FfiError::new(code, message));
    ptr::null_mut()
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
    frost_engine_new_in_memory()
}

#[unsafe(no_mangle)]
pub extern "C" fn frost_engine_new_in_memory() -> *mut FrostEngineHandle {
    clear_last_error();
    new_engine_handle(frost_core::InMemoryStore::default())
}

/// Creates a new FrostEngine instance backed by a SQLite store at `path`.
///
/// # Safety
///
/// - `path` must be a valid null-terminated UTF-8 string, or null.
/// - Returns a valid non-null handle on success.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frost_engine_new_with_store(
    path: *const c_char,
) -> *mut FrostEngineHandle {
    clear_last_error();
    if path.is_null() {
        return init_failed("invalid_argument", "SQLite store path was null");
    }
    let path = match unsafe { CStr::from_ptr(path) }.to_str() {
        Ok(path) if !path.is_empty() => path,
        Ok(_) => return init_failed("invalid_argument", "SQLite store path was empty"),
        Err(error) => return init_failed("invalid_utf8", error.to_string()),
    };
    match SqliteStore::open(path) {
        Ok(store) => new_engine_handle(store),
        Err(error) => init_failed(
            "store_open_failed",
            format!("failed to open SQLite store: {error}"),
        ),
    }
}

fn new_engine_handle<S>(store: S) -> *mut FrostEngineHandle
where
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
    let (request_tx, request_rx) = crossbeam_channel::unbounded();
    let (response_tx, response_rx) = crossbeam_channel::unbounded();
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let (host_command_tx, host_command_rx) = crossbeam_channel::unbounded();
    let (host_event_tx, host_event_rx) = crossbeam_channel::unbounded();
    let (host_result_tx, host_result_rx) = crossbeam_channel::unbounded();
    let (external_event_tx, _external_event_rx) = crossbeam_channel::unbounded();
    let (shutdown_tx, shutdown_rx) = crossbeam_channel::bounded(1);
    let (worker_done_tx, worker_done_rx) = crossbeam_channel::bounded(1);
    let state = Arc::new(AtomicU8::new(ENGINE_RUNNING));

    let join_handle = spawn_core(
        BrowserCore::with_adapter_and_settings(HostCommandAdapter::new(host_command_tx), store),
        event_tx,
        request_rx,
        response_tx,
        host_event_rx,
        host_result_rx,
        shutdown_rx,
        worker_done_tx,
        Arc::clone(&state),
    );

    Box::into_raw(Box::new(FrostEngineHandle {
        request_tx,
        response_rx,
        event_rx,
        host_command_rx,
        host_event_tx,
        host_result_tx,
        external_event_tx,
        external_policy: Mutex::new(ExternalPolicy::new()),
        request_response_lock: Mutex::new(()),
        shutdown_tx,
        worker_done_rx,
        state,
        next_request_id: AtomicU64::new(1),
        join_handle: Some(join_handle),
    }))
}

#[allow(clippy::too_many_arguments)]
fn spawn_core<A, S>(
    mut core: BrowserCore<A, S>,
    event_tx: Sender<EventEnvelope>,
    request_rx: Receiver<ProtocolRequest>,
    response_tx: Sender<ProtocolResponse>,
    host_event_rx: Receiver<HostEventEnvelope>,
    host_result_rx: Receiver<HostCommandResultEnvelope>,
    shutdown_rx: Receiver<()>,
    worker_done_tx: Sender<()>,
    state: Arc<AtomicU8>,
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
                recv(shutdown_rx) -> _ => break,
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
        state.store(ENGINE_STOPPED, Ordering::Release);
        let _ = worker_done_tx.send(());
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
        handle.state.store(ENGINE_SHUTTING_DOWN, Ordering::Release);
        let _ = handle.shutdown_tx.try_send(());
        if let Some(join_handle) = handle.join_handle.take() {
            if handle.worker_done_rx.recv_timeout(SHUTDOWN_TIMEOUT).is_ok() {
                let _ = join_handle.join();
            } else {
                drop(join_handle);
            }
        }
    }
}

/// Returns and clears the most recent FFI initialization/string error on the
/// calling thread as `{ "code": string, "message": string }` JSON.
///
/// The returned string must be released with `frost_engine_string_free`.
#[unsafe(no_mangle)]
pub extern "C" fn frost_engine_take_last_error_json() -> *mut c_char {
    let error = LAST_ERROR.with(|slot| slot.borrow_mut().take());
    match error {
        Some(error) => into_c_string(error.json().to_string()),
        None => ptr::null_mut(),
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
        return error_response_c_string(
            None,
            FfiError::new("invalid_argument", "handle or request was null"),
        );
    }

    unsafe {
        let request_text = match CStr::from_ptr(request_json).to_str() {
            Ok(text) => text,
            Err(error) => {
                return error_response_c_string(
                    None,
                    FfiError::new("invalid_utf8", error.to_string()),
                );
            }
        };
        let request = match serde_json::from_str::<ProtocolRequest>(request_text) {
            Ok(request) => request,
            Err(error) => {
                return error_response_c_string(
                    None,
                    FfiError::new("invalid_request", error.to_string()),
                );
            }
        };

        let id = request.id.clone();
        let handle = &*handle;
        match send_request(handle, request, Instant::now() + REQUEST_TIMEOUT) {
            Ok(response) => match serde_json::to_string(&response) {
                Ok(json) => response_c_string(id, json),
                Err(e) => {
                    eprintln!("[frost-ffi] Failed to serialize response: {e}");
                    error_response_c_string(
                        id,
                        FfiError::new("serialization_failed", "response serialization failed"),
                    )
                }
            },
            Err(error) => {
                eprintln!("[frost-ffi] request failed: {}", error.message);
                error_response_c_string(id, error)
            }
        }
    }
}

fn send_request(
    handle: &FrostEngineHandle,
    mut request: ProtocolRequest,
    deadline: Instant,
) -> Result<ProtocolResponse, FfiError> {
    if handle.state.load(Ordering::Acquire) != ENGINE_RUNNING {
        return Err(FfiError::new(
            "engine_stopped",
            "engine worker is not running",
        ));
    }
    let _guard = lock_until(&handle.request_response_lock, deadline)?;
    if handle.state.load(Ordering::Acquire) != ENGINE_RUNNING {
        return Err(FfiError::new("engine_stopped", "engine worker stopped"));
    }

    // Every FFI request gets an internal correlation id. A request that times
    // out can still complete on the single core worker later; without a unique
    // id, its late response would be mistaken for the next caller's response.
    // Restore the caller's id before returning so this remains an internal
    // implementation detail (including for requests that omitted an id).
    let caller_id = request.id.take();
    let internal_id = format!(
        "frost-ffi-{}",
        handle.next_request_id.fetch_add(1, Ordering::Relaxed)
    );
    request.id = Some(internal_id.clone());
    handle.request_tx.send(request).map_err(|_| {
        handle.state.store(ENGINE_STOPPED, Ordering::Release);
        FfiError::new("engine_stopped", "engine request channel is disconnected")
    })?;

    loop {
        match handle
            .response_rx
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            Ok(mut response) if response.id.as_deref() == Some(internal_id.as_str()) => {
                response.id = caller_id;
                return Ok(response);
            }
            // Discard a late response from a request whose caller already
            // timed out, then continue waiting for this request's response.
            Ok(_) => continue,
            Err(RecvTimeoutError::Timeout) => {
                return Err(FfiError::new(
                    "response_timeout",
                    "engine response timed out",
                ));
            }
            Err(RecvTimeoutError::Disconnected) => {
                handle.state.store(ENGINE_STOPPED, Ordering::Release);
                return Err(FfiError::new(
                    "response_channel_disconnected",
                    "engine response channel is disconnected",
                ));
            }
        }
    }
}

fn lock_until<'a, T>(
    mutex: &'a Mutex<T>,
    deadline: Instant,
) -> Result<MutexGuard<'a, T>, FfiError> {
    loop {
        match mutex.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(TryLockError::Poisoned(_)) => {
                return Err(FfiError::new("mutex_poisoned", "request mutex is poisoned"));
            }
            Err(TryLockError::WouldBlock) if Instant::now() >= deadline => {
                return Err(FfiError::new("response_timeout", "request mutex timed out"));
            }
            Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(1)),
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

fn into_c_string(value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(value) => value.into_raw(),
        Err(error) => {
            set_last_error(FfiError::new("c_string_failed", error.to_string()));
            ptr::null_mut()
        }
    }
}

fn response_c_string(id: Option<String>, value: String) -> *mut c_char {
    match CString::new(value) {
        Ok(value) => value.into_raw(),
        Err(error) => {
            error_response_c_string(id, FfiError::new("c_string_failed", error.to_string()))
        }
    }
}

fn error_response_c_string(id: Option<String>, error: FfiError) -> *mut c_char {
    set_last_error(error.clone());
    let json = serde_json::json!({
        "version": frost_protocol::PROTOCOL_VERSION, "id": id, "ok": false,
        "kind": "error", "result": error.json(),
    });
    CString::new(json.to_string())
        .expect("JSON escaped interior NUL")
        .into_raw()
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
    let response = route_external_to_core(
        handle,
        envelope,
        &mut policy,
        Instant::now() + REQUEST_TIMEOUT,
    );
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
    deadline: Instant,
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
            match send_request(handle, req, deadline) {
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
                Err(error) => {
                    let _ =
                        handle
                            .external_event_tx
                            .send(frost_protocol::ExternalEventEnvelope::new(
                                frost_protocol::ExternalEvent::Audit {
                                    command_id: envelope.id.clone(),
                                    capability,
                                    allowed: false,
                                    reason: Some(error.message.clone()),
                                },
                            ));
                    serde_json::json!({ "allowed": false, "error": error.json() }).to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    fn json(value: *mut c_char) -> serde_json::Value {
        assert!(!value.is_null());
        let result = unsafe { CStr::from_ptr(value) }.to_str().unwrap();
        let parsed = serde_json::from_str(result).unwrap();
        unsafe { frost_engine_string_free(value) };
        parsed
    }

    fn process(handle: *mut FrostEngineHandle) -> serde_json::Value {
        let request = CString::new(r#"{"version":0,"method":"app.snapshot"}"#).unwrap();
        json(unsafe { frost_engine_process_json(handle, request.as_ptr()) })
    }

    fn fake_handle() -> (
        Box<FrostEngineHandle>,
        Receiver<ProtocolRequest>,
        Sender<ProtocolResponse>,
    ) {
        let (request_tx, request_rx) = crossbeam_channel::unbounded();
        let (response_tx, response_rx) = crossbeam_channel::unbounded();
        let (_event_tx, event_rx) = crossbeam_channel::unbounded();
        let (_host_command_tx, host_command_rx) = crossbeam_channel::unbounded();
        let (host_event_tx, _host_event_rx) = crossbeam_channel::unbounded();
        let (host_result_tx, _host_result_rx) = crossbeam_channel::unbounded();
        let (external_event_tx, _external_event_rx) = crossbeam_channel::unbounded();
        let (shutdown_tx, _shutdown_rx) = crossbeam_channel::bounded(1);
        let (_worker_done_tx, worker_done_rx) = crossbeam_channel::bounded(1);

        (
            Box::new(FrostEngineHandle {
                request_tx,
                response_rx,
                event_rx,
                host_command_rx,
                host_event_tx,
                host_result_tx,
                external_event_tx,
                external_policy: Mutex::new(ExternalPolicy::new()),
                request_response_lock: Mutex::new(()),
                shutdown_tx,
                worker_done_rx,
                state: Arc::new(AtomicU8::new(ENGINE_RUNNING)),
                next_request_id: AtomicU64::new(1),
                join_handle: None,
            }),
            request_rx,
            response_tx,
        )
    }

    #[test]
    fn explicit_in_memory_constructor_processes_requests() {
        let handle = frost_engine_new_in_memory();
        assert!(!handle.is_null());
        let response = process(handle);
        assert_eq!(response["ok"], true);
        unsafe { frost_engine_free(handle) };
    }

    #[test]
    fn persistent_constructor_validates_path() {
        assert!(unsafe { frost_engine_new_with_store(ptr::null()) }.is_null());
        assert_eq!(
            json(frost_engine_take_last_error_json())["code"],
            "invalid_argument"
        );

        let empty = CString::new("").unwrap();
        assert!(unsafe { frost_engine_new_with_store(empty.as_ptr()) }.is_null());
        let invalid = [0xff_u8 as c_char, 0];
        assert!(unsafe { frost_engine_new_with_store(invalid.as_ptr()) }.is_null());
        assert_eq!(
            json(frost_engine_take_last_error_json())["code"],
            "invalid_utf8"
        );
    }

    #[test]
    fn persistent_constructor_rejects_unusable_targets() {
        for target in ["/dev/null", "/dev/null/frost.sqlite3"] {
            let path = CString::new(target).unwrap();
            assert!(unsafe { frost_engine_new_with_store(path.as_ptr()) }.is_null());
            assert_eq!(
                json(frost_engine_take_last_error_json())["code"],
                "store_open_failed"
            );
        }
    }

    #[test]
    fn unresponsive_worker_returns_structured_timeout_without_stopping_engine() {
        let (mut handle, _request_rx, _response_tx) = fake_handle();
        let started = Instant::now();
        let response = process(&mut *handle);
        assert_eq!(response["result"]["code"], "response_timeout");
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(handle.state.load(Ordering::Acquire), ENGINE_RUNNING);
    }

    #[test]
    fn request_after_timeout_discards_late_response_and_recovers() {
        let (mut handle, request_rx, response_tx) = fake_handle();
        let (release_late_response_tx, release_late_response_rx) = crossbeam_channel::bounded(1);
        let (late_response_sent_tx, late_response_sent_rx) = crossbeam_channel::bounded(1);
        let worker = std::thread::spawn(move || {
            let first = request_rx.recv().unwrap();
            release_late_response_rx.recv().unwrap();
            response_tx
                .send(ProtocolResponse::ok(
                    first.id,
                    frost_protocol::Response::Json(serde_json::json!({ "attempt": 1 })),
                ))
                .unwrap();
            late_response_sent_tx.send(()).unwrap();

            let second = request_rx.recv().unwrap();
            response_tx
                .send(ProtocolResponse::ok(
                    second.id,
                    frost_protocol::Response::Json(serde_json::json!({ "attempt": 2 })),
                ))
                .unwrap();
        });

        let first = process(&mut *handle);
        assert_eq!(first["result"]["code"], "response_timeout");
        assert_eq!(handle.state.load(Ordering::Acquire), ENGINE_RUNNING);

        release_late_response_tx.send(()).unwrap();
        late_response_sent_rx.recv().unwrap();
        let second = process(&mut *handle);
        assert_eq!(second["ok"], true);
        assert_eq!(second["result"]["attempt"], 2);
        assert!(second.get("id").is_none());
        worker.join().unwrap();
    }

    #[test]
    fn stopped_worker_request_fails_without_blocking() {
        let (mut handle, request_rx, _response_tx) = fake_handle();
        drop(request_rx);
        let started = Instant::now();
        let response = process(&mut *handle);
        assert_eq!(response["result"]["code"], "engine_stopped");
        assert!(started.elapsed() < Duration::from_millis(50));
    }

    #[test]
    fn disconnected_response_channel_is_structured_error() {
        let (mut handle, _request_rx, response_tx) = fake_handle();
        drop(response_tx);
        let response = process(&mut *handle);
        assert_eq!(response["result"]["code"], "response_channel_disconnected");
    }

    #[test]
    fn poisoned_request_mutex_is_structured_error() {
        let (mut handle, _request_rx, _response_tx) = fake_handle();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            let _guard = handle.request_response_lock.lock().unwrap();
            panic!("poison request mutex");
        }));
        let response = process(&mut *handle);
        assert_eq!(response["result"]["code"], "mutex_poisoned");
    }

    #[test]
    fn normal_free_completes_within_two_seconds() {
        let handle = frost_engine_new_in_memory();
        assert!(!handle.is_null());
        let started = Instant::now();
        unsafe { frost_engine_free(handle) };
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[test]
    fn c_string_failure_returns_structured_error_instead_of_empty_string() {
        let response = json(response_c_string(None, "contains\0nul".to_owned()));
        assert_eq!(response["result"]["code"], "c_string_failed");
    }
}
