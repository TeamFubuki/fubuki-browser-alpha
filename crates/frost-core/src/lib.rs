mod bookmark_service;
mod download_service;
mod external_router;
mod history_service;
mod settings_service;
mod tab_service;
mod window_service;

pub use external_router::{ExternalPolicy, ExternalResponse};

use std::{
    collections::HashMap,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crossbeam_channel::{Receiver, Sender};
use frost_engine_api::{EngineAdapter, EngineError, EngineResult, HostDispatch, NoopEngineAdapter};
use frost_protocol::{
    BrowserCommand, Event, EventEnvelope, HostCommand, HostCommandEnvelope,
    HostCommandResultEnvelope, HostEvent, HostEventEnvelope, OperationCompleted, OperationResponse,
    ProtocolRequest, ProtocolResponse, Request, Response, SettingChanged, TabActivated, TabClosed,
    TabPatch,
};
use frost_store::{
    BookmarkRepository, ClearRepository, DownloadRepository, HistoryRepository, LogRepository,
    PermissionRepository, SessionRepository, SettingsRepository,
};
use thiserror::Error;

pub use bookmark_service::BookmarkService;
pub use download_service::DownloadService;
pub use history_service::HistoryService;
pub use settings_service::SettingsService;
pub use tab_service::TabService;
pub use window_service::WindowService;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("{0}")]
    Message(String),
}

pub type CoreResult<T> = Result<T, CoreError>;

pub struct HostCommandAdapter {
    tx: Sender<HostCommandEnvelope>,
}

impl HostCommandAdapter {
    pub fn new(tx: Sender<HostCommandEnvelope>) -> Self {
        Self { tx }
    }

    fn send(&self, command: HostCommand) -> EngineResult<HostDispatch> {
        let operation_id = format!("operation-{}", uuid::Uuid::new_v4());
        self.tx
            .send(HostCommandEnvelope::new(operation_id.clone(), command))
            .map_err(|e| EngineError::Message(e.to_string()))?;
        Ok(HostDispatch::Queued { operation_id })
    }
}

impl EngineAdapter for HostCommandAdapter {
    fn create_page(
        &mut self,
        tab_id: &str,
        window_id: &str,
        url: &str,
    ) -> EngineResult<HostDispatch> {
        self.send(HostCommand::PageCreate {
            tab_id: tab_id.to_owned(),
            window_id: window_id.to_owned(),
            url: url.to_owned(),
        })
    }

    fn close_page(&mut self, tab_id: &str) -> EngineResult<HostDispatch> {
        self.send(HostCommand::PageClose {
            tab_id: tab_id.to_owned(),
        })
    }

    fn navigate(&mut self, tab_id: &str, input: &str) -> EngineResult<HostDispatch> {
        self.send(HostCommand::PageNavigate {
            tab_id: tab_id.to_owned(),
            url: input.to_owned(),
        })
    }

    fn reload(&mut self, tab_id: &str) -> EngineResult<HostDispatch> {
        self.send(HostCommand::PageReload {
            tab_id: tab_id.to_owned(),
        })
    }

    fn stop(&mut self, tab_id: &str) -> EngineResult<HostDispatch> {
        self.send(HostCommand::PageStop {
            tab_id: tab_id.to_owned(),
        })
    }

    fn go_back(&mut self, tab_id: &str) -> EngineResult<HostDispatch> {
        self.send(HostCommand::PageGoBack {
            tab_id: tab_id.to_owned(),
        })
    }

    fn go_forward(&mut self, tab_id: &str) -> EngineResult<HostDispatch> {
        self.send(HostCommand::PageGoForward {
            tab_id: tab_id.to_owned(),
        })
    }

    fn create_window(&mut self, window_id: &str, is_private: bool) -> EngineResult<HostDispatch> {
        self.send(HostCommand::WindowCreate {
            window_id: window_id.to_owned(),
            is_private,
        })
    }

    fn close_window(&mut self, window_id: &str) -> EngineResult<HostDispatch> {
        self.send(HostCommand::WindowClose {
            window_id: window_id.to_owned(),
        })
    }

    fn create_private_runtime(&mut self) -> EngineResult<HostDispatch> {
        self.send(HostCommand::RuntimeCreatePrivate)
    }
}

pub struct BrowserCore<A = NoopEngineAdapter, S = InMemoryStore> {
    adapter: A,
    repository: S,
    tabs: TabService,
    windows: WindowService,
    closed_tabs: Vec<frost_protocol::TabState>,
    events: Vec<EventEnvelope>,
    event_tx: Option<Sender<EventEnvelope>>,
    pending_operations: HashMap<String, PendingHostOperation>,
}

#[derive(Debug, Clone)]
enum PendingOperation {
    CreateTab(frost_protocol::TabState),
    CloseTab { tab_id: String },
    NavigateTab { tab_id: String, input: String },
    StopTab { tab_id: String },
    CreateWindow(frost_protocol::WindowState),
    CloseWindow { window_id: String },
    BootstrapWindow { window_id: String },
    NoStateChange,
}

#[derive(Debug, Clone)]
struct PendingHostOperation {
    action: PendingOperation,
    deadline: Instant,
}

const HOST_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
const IDLE_TICK_INTERVAL: Duration = Duration::from_secs(1);

impl BrowserCore<NoopEngineAdapter, InMemoryStore> {
    pub fn new() -> Self {
        Self::with_adapter_and_settings(NoopEngineAdapter, InMemoryStore::default())
    }
}

impl Default for BrowserCore<NoopEngineAdapter, InMemoryStore> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A, S> BrowserCore<A, S>
where
    A: EngineAdapter,
    S: SettingsRepository
        + BookmarkRepository
        + HistoryRepository
        + DownloadRepository
        + PermissionRepository
        + LogRepository
        + SessionRepository
        + ClearRepository,
{
    pub fn with_adapter_and_settings(adapter: A, repository: S) -> Self {
        Self::with_adapter_settings_and_initial_window(adapter, repository, false)
    }

    /// Constructs an engine with a Rust-owned initial window.  Private
    /// runtimes use this only with an in-memory repository; the `is_private`
    /// flag is therefore state owned by the same isolated FrostEngine.
    pub fn with_adapter_settings_and_initial_window(
        adapter: A,
        repository: S,
        is_private: bool,
    ) -> Self {
        let mut windows = WindowService::new();
        let window_id = windows.create_window(is_private);
        let tabs = TabService::new(window_id);

        Self {
            adapter,
            repository,
            tabs,
            windows,
            closed_tabs: Vec::new(),
            events: Vec::new(),
            event_tx: None,
            pending_operations: HashMap::new(),
        }
    }

    pub fn set_event_sender(&mut self, sender: Sender<EventEnvelope>) {
        self.event_tx = Some(sender);
    }

    /// Creates the Rust-owned initial window in the host. This is invoked by
    /// the runtime before it begins accepting bridge requests, so a failed
    /// bootstrap cannot leave an interactive phantom window behind.
    pub fn bootstrap_host(&mut self) -> CoreResult<()> {
        let window = self
            .windows
            .active_window_id()
            .and_then(|id| self.windows.get_window(id))
            .ok_or_else(|| CoreError::Message("No initial window to bootstrap".into()))?;
        let dispatch = self
            .adapter
            .create_window(&window.id, window.is_private)
            .map_err(|error| CoreError::Message(error.to_string()))?;
        self.resolve_host_dispatch(
            dispatch,
            PendingOperation::BootstrapWindow {
                window_id: window.id,
            },
        )
        .map(|_| ())
    }

    pub fn recent_events(&self) -> &[EventEnvelope] {
        &self.events
    }

    pub fn process(&mut self, request: ProtocolRequest) -> ProtocolResponse {
        self.tick();
        let id = request.id.clone();
        if request.version != frost_protocol::PROTOCOL_VERSION {
            return ProtocolResponse::error(
                id,
                format!("Unsupported protocol version {}", request.version),
            );
        }
        match self.process_inner(request.request) {
            Ok(response) => ProtocolResponse::ok(id, response),
            Err(error) => ProtocolResponse::error(id, error.to_string()),
        }
    }

    /// Advances asynchronous host-operation deadlines without requiring an
    /// incoming bridge request, host event, or host result.  Runtimes must
    /// call this while idle (see frost-ffi's deadline-aware event loop).
    pub fn tick(&mut self) {
        self.expire_pending_operations_at(Instant::now());
    }

    /// The earliest pending-host deadline.  A runtime may use this to sleep
    /// until the precise next timeout instead of relying on UI traffic.
    pub fn next_pending_deadline(&self) -> Option<Instant> {
        self.pending_operations
            .values()
            .map(|pending| pending.deadline)
            .min()
    }

    pub fn idle_tick_interval() -> Duration {
        IDLE_TICK_INTERVAL
    }

    pub fn run(
        mut self,
        request_rx: Receiver<ProtocolRequest>,
        response_tx: Sender<ProtocolResponse>,
    ) {
        while let Ok(request) = request_rx.recv() {
            let response = self.process(request);
            if response_tx.send(response).is_err() {
                break;
            }
        }
    }

    fn process_inner(&mut self, request: Request) -> CoreResult<Response> {
        match request {
            Request::AppSnapshot => self.snapshot().map(Response::AppSnapshot),
            Request::TabsList => Ok(Response::TabsList(self.tabs.list())),
            Request::TabsCreate {
                url,
                active,
                window_id,
            } => {
                let window_id = match window_id {
                    Some(window_id) => {
                        if self.windows.get_window(&window_id).is_none() {
                            return Err(CoreError::Message(format!("Unknown window {window_id}")));
                        }
                        window_id
                    }
                    None => self
                        .windows
                        .active_window_id()
                        .ok_or_else(|| CoreError::Message("No active window".into()))?
                        .to_owned(),
                };
                let tab = self.tabs.new_tab(
                    window_id.clone(),
                    url.unwrap_or_else(|| "fubuki://newtab/".into()),
                    active,
                );
                let dispatch = self
                    .adapter
                    .create_page(&tab.id, &window_id, &tab.url)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.response_for_host_dispatch(dispatch, PendingOperation::CreateTab(tab))
            }
            Request::TabsActivate { tab_id } => {
                let activated = self.tabs.activate_tab(&tab_id);
                if activated {
                    self.windows.set_active_tab(&tab_id);
                    self.emit(Event::TabActivated(TabActivated { tab_id }));
                }
                Ok(Response::Bool(activated))
            }
            Request::TabsClose { tab_id } => {
                let Some(tab) = self.tabs.get_tab(&tab_id) else {
                    return Ok(Response::Bool(false));
                };
                // The host must never synthesize a replacement tab after a
                // close. Keep one Rust-owned page per live window until the
                // product defines an atomic close-window transaction.
                if self
                    .tabs
                    .list()
                    .into_iter()
                    .filter(|candidate| candidate.window_id == tab.window_id)
                    .count()
                    <= 1
                {
                    return Ok(Response::Bool(false));
                }
                let dispatch = self
                    .adapter
                    .close_page(&tab_id)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.response_for_host_dispatch(dispatch, PendingOperation::CloseTab { tab_id })
            }
            Request::TabsPin { tab_id, pinned } => {
                let ok = self.tabs.pin_tab(&tab_id, pinned);
                if ok {
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        is_pinned: Some(pinned),
                        ..Default::default()
                    }));
                }
                Ok(Response::Bool(ok))
            }
            Request::TabsDuplicate { tab_id } => {
                let Some(mut tab) = self.tabs.get_tab(&tab_id) else {
                    return Ok(Response::Bool(false));
                };
                tab.id = format!("tab-{}", uuid::Uuid::new_v4());
                tab.is_active = false;
                let dispatch = self
                    .adapter
                    .create_page(&tab.id, &tab.window_id, &tab.url)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.response_for_host_dispatch(dispatch, PendingOperation::CreateTab(tab))
            }
            Request::TabsReopenClosed => Err(CoreError::Message(
                "Reopening a closed tab is unavailable until host restoration is transactional"
                    .into(),
            )),
            Request::TabsCloseOther { tab_id } => {
                let selected = self
                    .tabs
                    .get_tab(&tab_id)
                    .ok_or_else(|| CoreError::Message(format!("Unknown tab {tab_id}")))?;
                let to_close = self
                    .tabs
                    .list()
                    .into_iter()
                    .filter(|tab| tab.window_id == selected.window_id && tab.id != tab_id)
                    .map(|tab| tab.id)
                    .collect::<Vec<_>>();
                let mut pending = Vec::new();
                for close_id in to_close {
                    let dispatch = self
                        .adapter
                        .close_page(&close_id)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    if let Some(operation_id) = self.resolve_host_dispatch(
                        dispatch,
                        PendingOperation::CloseTab { tab_id: close_id },
                    )? {
                        pending.push(OperationResponse::pending(operation_id));
                    }
                }
                Ok(match pending.len() {
                    0 => Response::Bool(true),
                    1 => Response::Operation(pending.remove(0)),
                    _ => Response::Operations(pending),
                })
            }
            Request::TabsCloseToRight { tab_id } => Err(CoreError::Message(format!(
                "Closing tabs to the right of {tab_id} is unavailable until tab ordering is host-confirmed"
            ))),
            Request::TabsMove { tab_id, to_index } => {
                let ok = self.tabs.move_tab(&tab_id, to_index);
                if ok {
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        ..Default::default()
                    }));
                }
                Ok(Response::Bool(ok))
            }
            Request::TabsMoveToNewWindow { tab_id } => Err(CoreError::Message(format!(
                "Moving tab {tab_id} to a new window is unavailable until the host can reparent pages atomically"
            ))),
            Request::TabsNavigate { tab_id, input } => {
                if !self.tabs.contains(&tab_id) {
                    return Ok(Response::Bool(false));
                }
                let dispatch = self
                    .adapter
                    .navigate(&tab_id, &input)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.response_for_host_dispatch(
                    dispatch,
                    PendingOperation::NavigateTab { tab_id, input },
                )
            }
            Request::TabsReload { tab_id } => self.host_tab_action(&tab_id, HostTabAction::Reload),
            Request::TabsStop { tab_id } => {
                if !self.tabs.contains(&tab_id) {
                    return Ok(Response::Bool(false));
                }
                let dispatch = self
                    .adapter
                    .stop(&tab_id)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.response_for_host_dispatch(dispatch, PendingOperation::StopTab { tab_id })
            }
            Request::TabsGoBack { tab_id } => self.host_tab_action(&tab_id, HostTabAction::GoBack),
            Request::TabsGoForward { tab_id } => {
                self.host_tab_action(&tab_id, HostTabAction::GoForward)
            }
            Request::TabsHome => {
                let Some(tab) = self.tabs.active_tab() else {
                    return Ok(Response::Bool(false));
                };
                let home = SettingsService::get(&self.repository, "homeUrl")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "fubuki://newtab/".into());
                self.process_inner(Request::TabsNavigate {
                    tab_id: tab.id,
                    input: home,
                })
            }
            Request::WindowsList => Ok(Response::WindowsList(self.windows.list())),
            Request::WindowsCreate => {
                let window = WindowService::new_window(false);
                let dispatch = self
                    .adapter
                    .create_window(&window.id, false)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.response_for_host_dispatch(dispatch, PendingOperation::CreateWindow(window))
            }
            Request::WindowsCreatePrivate => {
                let dispatch = self
                    .adapter
                    .create_private_runtime()
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                // The persistent engine cannot own a private window ID or
                // state. The host creates a fresh in-memory FrostRuntime and
                // returns only this operation's terminal result.
                self.response_for_host_dispatch(dispatch, PendingOperation::NoStateChange)
            }
            Request::WindowsClose { window_id } => {
                let target = window_id
                    .or_else(|| self.windows.active_window_id().map(ToOwned::to_owned))
                    .ok_or_else(|| CoreError::Message("No active window".into()))?;
                if self.windows.get_window(&target).is_none() {
                    return Ok(Response::Bool(false));
                }
                let dispatch = self
                    .adapter
                    .close_window(&target)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.response_for_host_dispatch(
                    dispatch,
                    PendingOperation::CloseWindow { window_id: target },
                )
            }
            Request::WindowsReopenClosed => Err(CoreError::Message(
                "Reopening a closed window is unavailable until host restoration is transactional"
                    .into(),
            )),
            Request::SettingsGet { key } => SettingsService::get(&self.repository, &key)
                .map(Response::Setting)
                .map_err(|e| CoreError::Message(e.to_string())),
            Request::SettingsSet { key, value } => {
                SettingsService::set(&self.repository, &key, &value)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.emit(Event::SettingChanged(SettingChanged { key, value }));
                Ok(Response::Bool(true))
            }
            Request::SettingsReset { key } => {
                let value = SettingsService::default_value(&key);
                SettingsService::set(&self.repository, &key, &value)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.emit(Event::SettingChanged(SettingChanged { key, value }));
                Ok(Response::Bool(true))
            }
            Request::BookmarksList => BookmarkService::list(&self.repository)
                .map(Response::BookmarksList)
                .map_err(|e| CoreError::Message(e.to_string())),
            Request::BookmarksSave {
                title,
                url,
                favicon_url,
            } => {
                let ok = BookmarkService::save(
                    &self.repository,
                    &title,
                    &url,
                    favicon_url.as_deref().unwrap_or_default(),
                )
                .map_err(|e| CoreError::Message(e.to_string()))?;
                if ok {
                    self.emit(Event::BookmarkChanged { url });
                }
                Ok(Response::Bool(ok))
            }
            Request::BookmarksRemove { url } => {
                let ok = BookmarkService::remove(&self.repository, &url)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                if ok {
                    self.emit(Event::BookmarkChanged { url });
                }
                Ok(Response::Bool(ok))
            }
            Request::HistoryList => HistoryService::list(&self.repository)
                .map(Response::HistoryList)
                .map_err(|e| CoreError::Message(e.to_string())),
            Request::HistoryRemove { url } => {
                let ok = HistoryService::remove(&self.repository, &url)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                if ok {
                    self.emit(Event::HistoryChanged { url: Some(url) });
                }
                Ok(Response::Bool(ok))
            }
            Request::HistoryClearRange { range } => {
                let ok = HistoryService::clear_range(&self.repository, &range)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                if ok {
                    self.emit(Event::HistoryChanged { url: None });
                }
                Ok(Response::Bool(ok))
            }
            Request::DownloadsList => DownloadService::list(&self.repository)
                .map(Response::DownloadsList)
                .map_err(|e| CoreError::Message(e.to_string())),
            Request::DownloadsRemove { url, path } => {
                let ok = DownloadService::remove(&self.repository, url.as_deref(), path.as_deref())
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                if ok {
                    self.emit(Event::DownloadChanged {
                        url,
                        path,
                        state: None,
                        percent: None,
                    });
                }
                Ok(Response::Bool(ok))
            }
            Request::DataClear { target } => {
                let target = target.unwrap_or_else(|| "all".into());
                if target == "bookmarks" || target == "all" {
                    self.repository
                        .clear_bookmarks()
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::BookmarkChanged { url: String::new() });
                }
                if target == "history" || target == "all" {
                    HistoryService::clear_range(&self.repository, "all")
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::HistoryChanged { url: None });
                }
                if target == "downloads" || target == "all" {
                    self.repository
                        .clear_downloads()
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::DownloadChanged {
                        url: None,
                        path: None,
                        state: None,
                        percent: None,
                    });
                }
                Ok(Response::Bool(true))
            }
            Request::PermissionsSet {
                origin,
                permission,
                value,
            } => {
                self.repository
                    .set_permission(&origin, &permission, &value)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.emit(Event::PermissionChanged { origin, permission });
                Ok(Response::Bool(true))
            }
            Request::CommandsList => Ok(Response::CommandsList(default_commands())),
            Request::CommandsExecute { id, args: _ } => self.execute_command(&id),
            Request::UiSetSidebarWidth { width } => {
                SettingsService::set(
                    &self.repository,
                    "sidebarWidth",
                    &format!("{}", width.round()),
                )
                .map_err(|e| CoreError::Message(e.to_string()))?;
                Ok(Response::Bool(true))
            }
            Request::UiSetOverlayActive {
                active: _,
                width: _,
                height: _,
            } => Ok(Response::Bool(true)),
            Request::HostSyncSnapshot { .. } => Err(CoreError::Message(
                "host.syncSnapshot is disabled: FrostEngine is the only state authority".into(),
            )),
        }
    }

    pub fn process_host_event(&mut self, envelope: HostEventEnvelope) -> CoreResult<()> {
        self.tick();
        if envelope.version != frost_protocol::PROTOCOL_VERSION {
            return Err(CoreError::Message(format!(
                "Unsupported host event protocol version {}",
                envelope.version
            )));
        }
        match envelope.event {
            HostEvent::PageCreated {
                tab_id,
                window_id,
                url,
            } => {
                if let Some(mut tab) = self.tabs.get_tab(&tab_id) {
                    tab.window_id = window_id.clone();
                    tab.url = url;
                    self.tabs.upsert_tab(tab);
                    self.windows.attach_tab(&window_id, &tab_id, false);
                    Ok(())
                } else if self.pending_operations.values().any(|pending| {
                    matches!(&pending.action, PendingOperation::CreateTab(tab) if tab.id == tab_id)
                }) {
                    // The terminal HostCommandResult is the commit point. A
                    // lifecycle event is allowed to race ahead of it.
                    Ok(())
                } else {
                    Err(CoreError::Message(format!("Host created unknown tab {tab_id}")))
                }
            }
            HostEvent::PageClosed { tab_id } => {
                if self.pending_operations.values().any(|pending| {
                    matches!(&pending.action, PendingOperation::CloseTab { tab_id: pending_id } if pending_id == &tab_id)
                }) {
                    return Ok(());
                }
                let closed = self.tabs.close_tab(&tab_id);
                if closed {
                    self.windows.detach_tab(&tab_id);
                    self.emit(Event::TabClosed(TabClosed { tab_id }));
                    Ok(())
                } else {
                    Err(CoreError::Message(format!(
                        "Host closed unknown tab {tab_id}"
                    )))
                }
            }
            HostEvent::PageTitleChanged { tab_id, title } => {
                if self.tabs.set_title(&tab_id, &title) {
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        title: Some(title),
                        ..Default::default()
                    }));
                }
                Ok(())
            }
            HostEvent::PageUrlChanged { tab_id, url } => {
                if self.tabs.set_url(&tab_id, &url) {
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        url: Some(url),
                        ..Default::default()
                    }));
                }
                Ok(())
            }
            HostEvent::PageFaviconChanged {
                tab_id,
                favicon_url,
            } => {
                if self.tabs.set_favicon_url(&tab_id, &favicon_url) {
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        favicon_url: Some(favicon_url),
                        ..Default::default()
                    }));
                }
                Ok(())
            }
            HostEvent::PageLoadingChanged { tab_id, is_loading } => {
                if self.tabs.set_loading(&tab_id, is_loading) {
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        is_loading: Some(is_loading),
                        ..Default::default()
                    }));
                }
                Ok(())
            }
            HostEvent::PageNavigationStateChanged {
                tab_id,
                can_go_back,
                can_go_forward,
            } => {
                if self
                    .tabs
                    .set_navigation_state(&tab_id, can_go_back, can_go_forward)
                {
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        can_go_back: Some(can_go_back),
                        can_go_forward: Some(can_go_forward),
                        ..Default::default()
                    }));
                }
                Ok(())
            }
            HostEvent::PageLoadFailed { tab_id, error_text } => {
                if self.tabs.set_error_text(&tab_id, &error_text) {
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        error_text: Some(error_text),
                        is_loading: Some(false),
                        ..Default::default()
                    }));
                }
                Ok(())
            }
            HostEvent::PageNavigationFinished { tab_id } => {
                let tab = self
                    .tabs
                    .get_tab(&tab_id)
                    .ok_or_else(|| CoreError::Message(format!("Unknown tab {tab_id}")))?;
                self.repository
                    .add_history(&tab.title, &tab.url, &tab.favicon_url)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.emit(Event::HistoryChanged { url: Some(tab.url) });
                Ok(())
            }
            HostEvent::DownloadUpdated {
                url,
                path,
                state,
                percent,
            } => {
                self.repository
                    .upsert_download(&url, &path, &state, percent)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.emit(Event::DownloadChanged {
                    url: Some(url),
                    path: Some(path),
                    state: Some(state),
                    percent: Some(percent),
                });
                Ok(())
            }
            HostEvent::PermissionChanged {
                origin,
                permission,
                value,
            } => {
                self.repository
                    .set_permission(&origin, &permission, &value)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.emit(Event::PermissionChanged { origin, permission });
                Ok(())
            }
            HostEvent::WindowFocused { window_id } => {
                self.windows.set_active_window(&window_id);
                Ok(())
            }
            HostEvent::WindowClosed { window_id } => {
                // NSWindow has closed, but a matching HostCommandResult is
                // still the sole commit point for a Rust-requested close.
                // Treat this lifecycle notice as confirmation only; the
                // controller will submit the terminal result afterwards.
                if self.pending_operations.values().any(|pending| {
                    matches!(
                        &pending.action,
                        PendingOperation::CloseWindow { window_id: pending_id }
                            if pending_id == &window_id
                    )
                }) {
                    return Ok(());
                }
                if self.windows.close_window(&window_id) {
                    let tab_ids: Vec<String> = self
                        .tabs
                        .list()
                        .into_iter()
                        .filter(|tab| tab.window_id == window_id)
                        .map(|tab| tab.id)
                        .collect();
                    for tab_id in tab_ids {
                        self.tabs.remove_tab(&tab_id);
                        self.emit(Event::TabClosed(TabClosed { tab_id }));
                    }
                    self.emit(Event::WindowClosed { window_id });
                }
                Ok(())
            }
        }
    }

    pub fn process_host_command_result(
        &mut self,
        result: HostCommandResultEnvelope,
    ) -> CoreResult<()> {
        self.tick();
        if result.version != frost_protocol::PROTOCOL_VERSION {
            return Err(CoreError::Message(format!(
                "Unsupported host result protocol version {}",
                result.version
            )));
        }
        let Some(pending) = self.pending_operations.remove(&result.operation_id) else {
            return Err(CoreError::Message(format!(
                "Host returned a result for unknown operation {}",
                result.operation_id
            )));
        };
        if result.ok {
            match self.commit_pending_operation(pending.action) {
                Ok(()) => {
                    self.emit(Event::HostOperationCompleted(
                        OperationCompleted::succeeded(result.operation_id),
                    ));
                    Ok(())
                }
                Err(error) => {
                    let message = error.to_string();
                    self.emit(Event::HostOperationCompleted(OperationCompleted::failed(
                        result.operation_id.clone(),
                        message.clone(),
                    )));
                    Err(CoreError::Message(format!(
                        "Host operation {} could not be committed: {message}",
                        result.operation_id
                    )))
                }
            }
        } else {
            let error = result.error.unwrap_or_else(|| "host command failed".into());
            self.record_failed_operation(
                result.operation_id.clone(),
                pending.action,
                error.clone(),
            );
            Err(CoreError::Message(format!(
                "Host operation {} failed: {error}",
                result.operation_id
            )))
        }
    }

    fn resolve_host_dispatch(
        &mut self,
        dispatch: HostDispatch,
        action: PendingOperation,
    ) -> CoreResult<Option<String>> {
        match dispatch {
            HostDispatch::Completed => {
                self.commit_pending_operation(action)?;
                Ok(None)
            }
            HostDispatch::Queued { operation_id } => {
                if self
                    .pending_operations
                    .insert(
                        operation_id.clone(),
                        PendingHostOperation {
                            action,
                            deadline: Instant::now() + HOST_OPERATION_TIMEOUT,
                        },
                    )
                    .is_some()
                {
                    return Err(CoreError::Message(format!(
                        "Duplicate host operation ID {operation_id}"
                    )));
                }
                Ok(Some(operation_id))
            }
        }
    }

    fn response_for_host_dispatch(
        &mut self,
        dispatch: HostDispatch,
        action: PendingOperation,
    ) -> CoreResult<Response> {
        Ok(match self.resolve_host_dispatch(dispatch, action)? {
            Some(operation_id) => Response::Operation(OperationResponse::pending(operation_id)),
            None => Response::Bool(true),
        })
    }

    fn commit_pending_operation(&mut self, action: PendingOperation) -> CoreResult<()> {
        match action {
            PendingOperation::CreateTab(tab) => {
                if self.tabs.contains(&tab.id) {
                    return Err(CoreError::Message(format!("Tab {} already exists", tab.id)));
                }
                self.tabs.commit_new_tab(tab.clone());
                self.windows
                    .attach_tab(&tab.window_id, &tab.id, tab.is_active);
                self.emit(Event::TabCreated(tab));
            }
            PendingOperation::CloseTab { tab_id } => {
                let tab = self.tabs.get_tab(&tab_id).ok_or_else(|| {
                    CoreError::Message(format!(
                        "Tab {tab_id} disappeared before host close completed"
                    ))
                })?;
                self.closed_tabs.push(tab);
                self.trim_closed_tabs();
                if !self.tabs.close_tab(&tab_id) {
                    return Err(CoreError::Message(format!("Unable to close tab {tab_id}")));
                }
                self.windows.detach_tab(&tab_id);
                self.emit(Event::TabClosed(TabClosed { tab_id }));
            }
            PendingOperation::NavigateTab { tab_id, input } => {
                if !self.tabs.navigate(&tab_id, &input) {
                    return Err(CoreError::Message(format!("Unknown tab {tab_id}")));
                }
                self.emit(Event::TabUpdated(TabPatch {
                    tab_id,
                    url: Some(input),
                    is_loading: Some(true),
                    ..Default::default()
                }));
            }
            PendingOperation::StopTab { tab_id } => {
                if !self.tabs.stop_tab(&tab_id) {
                    return Err(CoreError::Message(format!("Unknown tab {tab_id}")));
                }
                self.emit(Event::TabUpdated(TabPatch {
                    tab_id,
                    is_loading: Some(false),
                    ..Default::default()
                }));
            }
            PendingOperation::CreateWindow(window) => {
                if self.windows.get_window(&window.id).is_some() {
                    return Err(CoreError::Message(format!(
                        "Window {} already exists",
                        window.id
                    )));
                }
                let window_id = window.id.clone();
                self.windows.commit_new_window(window.clone());
                self.emit(Event::WindowCreated(window));
                self.queue_initial_tab(window_id)?;
            }
            PendingOperation::CloseWindow { window_id } => {
                self.windows.get_window(&window_id).ok_or_else(|| {
                    CoreError::Message(format!(
                        "Window {window_id} disappeared before host close completed"
                    ))
                })?;
                let tabs = self
                    .tabs
                    .list()
                    .into_iter()
                    .filter(|tab| tab.window_id == window_id)
                    .collect::<Vec<_>>();
                if !self.windows.close_window(&window_id) {
                    return Err(CoreError::Message(format!(
                        "Unable to close window {window_id}"
                    )));
                }
                for tab in tabs {
                    self.tabs.remove_tab(&tab.id);
                    self.emit(Event::TabClosed(TabClosed { tab_id: tab.id }));
                }
                self.emit(Event::WindowClosed { window_id });
            }
            PendingOperation::BootstrapWindow { window_id } => {
                // A host window without a Rust-owned initial page is not a
                // usable browser. Create that page through the same pending
                // operation path rather than letting BrowserWindow invent it.
                self.queue_initial_tab(window_id)?;
            }
            PendingOperation::NoStateChange => {}
        }
        Ok(())
    }

    fn queue_initial_tab(&mut self, window_id: String) -> CoreResult<()> {
        let tab = self
            .tabs
            .new_tab(window_id.clone(), "fubuki://newtab/".into(), true);
        let dispatch = self
            .adapter
            .create_page(&tab.id, &window_id, &tab.url)
            .map_err(|error| CoreError::Message(error.to_string()))?;
        self.resolve_host_dispatch(dispatch, PendingOperation::CreateTab(tab))?;
        Ok(())
    }

    fn expire_pending_operations_at(&mut self, now: Instant) {
        let expired = self
            .pending_operations
            .iter()
            .filter(|(_, pending)| now >= pending.deadline)
            .map(|(operation_id, _)| operation_id.clone())
            .collect::<Vec<_>>();
        for operation_id in expired {
            if let Some(pending) = self.pending_operations.remove(&operation_id) {
                self.record_failed_operation(
                    operation_id,
                    pending.action,
                    "host operation timed out".into(),
                );
            }
        }
    }

    fn record_failed_operation(
        &mut self,
        operation_id: String,
        action: PendingOperation,
        error: String,
    ) {
        if let PendingOperation::BootstrapWindow { window_id } = action {
            self.windows.close_window(&window_id);
        }
        let completion = if error == "host operation timed out" {
            OperationCompleted::timed_out(operation_id.clone())
        } else {
            OperationCompleted::failed(operation_id.clone(), error.clone())
        };
        self.emit(Event::HostOperationCompleted(completion));
    }

    fn host_tab_action(&mut self, tab_id: &str, action: HostTabAction) -> CoreResult<Response> {
        if !self.tabs.contains(tab_id) {
            return Ok(Response::Bool(false));
        }

        let result = match action {
            HostTabAction::Reload => self.adapter.reload(tab_id),
            HostTabAction::GoBack => self.adapter.go_back(tab_id),
            HostTabAction::GoForward => self.adapter.go_forward(tab_id),
        };
        let dispatch = result.map_err(|e| CoreError::Message(e.to_string()))?;
        self.response_for_host_dispatch(dispatch, PendingOperation::NoStateChange)
    }

    fn execute_command(&mut self, id: &str) -> CoreResult<Response> {
        let active_tab_id = || {
            self.tabs
                .active_tab()
                .map(|tab| tab.id)
                .ok_or_else(|| CoreError::Message("No active tab".into()))
        };
        let request = match id {
            "tabs.create" => Request::TabsCreate {
                url: None,
                active: true,
                window_id: None,
            },
            "tabs.close" => Request::TabsClose {
                tab_id: active_tab_id()?,
            },
            "tabs.reopenClosed" => Request::TabsReopenClosed,
            "tabs.duplicate" => Request::TabsDuplicate {
                tab_id: active_tab_id()?,
            },
            "tabs.pin" => Request::TabsPin {
                tab_id: active_tab_id()?,
                pinned: true,
            },
            "tabs.unpin" => Request::TabsPin {
                tab_id: active_tab_id()?,
                pinned: false,
            },
            "tabs.closeOther" => Request::TabsCloseOther {
                tab_id: active_tab_id()?,
            },
            "tabs.closeToRight" => Request::TabsCloseToRight {
                tab_id: active_tab_id()?,
            },
            "tabs.moveToNewWindow" => Request::TabsMoveToNewWindow {
                tab_id: active_tab_id()?,
            },
            "tabs.reload" => Request::TabsReload {
                tab_id: active_tab_id()?,
            },
            "tabs.stop" => Request::TabsStop {
                tab_id: active_tab_id()?,
            },
            "tabs.goBack" => Request::TabsGoBack {
                tab_id: active_tab_id()?,
            },
            "tabs.goForward" => Request::TabsGoForward {
                tab_id: active_tab_id()?,
            },
            "tabs.home" => Request::TabsHome,
            "windows.create" => Request::WindowsCreate,
            "windows.createPrivate" => Request::WindowsCreatePrivate,
            "windows.close" => Request::WindowsClose { window_id: None },
            "windows.reopenClosed" => Request::WindowsReopenClosed,
            "bookmarks.addActive" => {
                let tab = self
                    .tabs
                    .active_tab()
                    .ok_or_else(|| CoreError::Message("No active tab".into()))?;
                Request::BookmarksSave {
                    title: tab.title,
                    url: tab.url,
                    favicon_url: Some(tab.favicon_url),
                }
            }
            _ => {
                return Err(CoreError::Message(format!(
                    "Command '{id}' is unknown or not implemented by FrostEngine"
                )));
            }
        };
        let response = self.process_inner(request)?;
        // Pending host operations must remain top-level protocol operations.
        // Nesting them in a command metadata JSON object makes NativeBridge
        // resolve the renderer Promise before the terminal Host result.
        if matches!(response, Response::Operation(_) | Response::Operations(_)) {
            return Ok(response);
        }
        Ok(Response::Json(serde_json::json!({
            "handled": true,
            "id": id,
            "result": response,
        })))
    }

    fn snapshot(&self) -> CoreResult<frost_protocol::AppState> {
        let settings = self.build_settings_snapshot()?;
        Ok(frost_protocol::AppState {
            protocol_version: frost_protocol::PROTOCOL_VERSION,
            active_window_id: self.windows.active_window_id().map(ToOwned::to_owned),
            windows: self.windows.list(),
            tabs: self.tabs.list(),
            history: self
                .repository
                .list_history()
                .map_err(|e| CoreError::Message(e.to_string()))?,
            bookmarks: self
                .repository
                .list_bookmarks()
                .map_err(|e| CoreError::Message(e.to_string()))?,
            downloads: self
                .repository
                .list_downloads()
                .map_err(|e| CoreError::Message(e.to_string()))?,
            permissions: self
                .repository
                .list_permissions()
                .map_err(|e| CoreError::Message(e.to_string()))?,
            settings,
        })
    }

    fn build_settings_snapshot(&self) -> CoreResult<serde_json::Value> {
        let mut map = serde_json::Map::new();
        for key in SettingsService::VALID_KEYS {
            let value = self
                .repository
                .get_setting(key)
                .map_err(|e| CoreError::Message(e.to_string()))?
                .unwrap_or_else(|| SettingsService::default_value(key));
            map.insert((*key).to_owned(), serde_json::Value::String(value));
        }
        Ok(serde_json::Value::Object(map))
    }

    fn emit(&mut self, event: Event) {
        let envelope = EventEnvelope::new(event);
        self.events.push(envelope.clone());
        if self.events.len() > 100 {
            self.events.remove(0);
        }
        if let Some(sender) = &self.event_tx
            && let Err(error) = sender.send(envelope)
        {
            eprintln!("[frost-engine] event receiver disconnected: {error}");
        }
    }

    fn trim_closed_tabs(&mut self) {
        if self.closed_tabs.len() > 50 {
            let excess = self.closed_tabs.len() - 50;
            self.closed_tabs.drain(..excess);
        }
    }
}

fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn default_commands() -> Vec<BrowserCommand> {
    [
        ("tabs.create", "New Tab", "Tabs", "Cmd+T"),
        ("tabs.close", "Close Tab", "Tabs", "Cmd+W"),
        ("tabs.reopenClosed", "Reopen Closed Tab", "Tabs", ""),
        ("tabs.duplicate", "Duplicate Tab", "Tabs", ""),
        ("tabs.pin", "Pin Tab", "Tabs", ""),
        ("tabs.unpin", "Unpin Tab", "Tabs", ""),
        ("tabs.closeOther", "Close Other Tabs", "Tabs", ""),
        ("tabs.closeToRight", "Close Tabs to Right", "Tabs", ""),
        ("tabs.moveToNewWindow", "Move Tab to New Window", "Tabs", ""),
        ("tabs.reload", "Reload", "Tabs", "Cmd+R"),
        ("tabs.stop", "Stop Loading", "Tabs", ""),
        ("tabs.goBack", "Back", "Navigation", "Cmd+["),
        ("tabs.goForward", "Forward", "Navigation", "Cmd+]"),
        ("tabs.home", "Home", "Navigation", ""),
        ("windows.create", "New Window", "Window", "Cmd+N"),
        ("windows.createPrivate", "New Private Window", "Window", ""),
        ("windows.close", "Close Window", "Window", ""),
        ("windows.reopenClosed", "Reopen Closed Window", "Window", ""),
        ("app.focusOmnibox", "Focus Omnibox", "App", "Cmd+L"),
        ("app.openSettings", "Open Settings", "App", ""),
        ("app.openHistory", "Open History", "App", ""),
        ("app.openDownloads", "Open Downloads", "App", ""),
        ("app.openBookmarks", "Open Bookmarks", "App", ""),
        ("app.openDevTools", "Open DevTools", "Developer", ""),
        ("page.find", "Find in Page", "Page", "Cmd+F"),
        ("page.zoomIn", "Zoom In", "Page", "Cmd++"),
        ("page.zoomOut", "Zoom Out", "Page", "Cmd+-"),
        ("page.zoomReset", "Actual Size", "Page", "Cmd+0"),
        ("page.print", "Print", "Page", "Cmd+P"),
        ("page.viewSource", "View Source", "Developer", ""),
        (
            "bookmarks.addActive",
            "Bookmark Active Tab",
            "Bookmarks",
            "Cmd+D",
        ),
        ("bookmarks.save", "Save Bookmark", "Bookmarks", ""),
        ("bookmarks.remove", "Remove Bookmark", "Bookmarks", ""),
    ]
    .into_iter()
    .map(|(id, title, category, shortcut)| BrowserCommand {
        id: id.into(),
        title: title.into(),
        category: category.into(),
        shortcut: shortcut.into(),
    })
    .collect()
}

enum HostTabAction {
    Reload,
    GoBack,
    GoForward,
}

#[derive(Default)]
pub struct InMemoryStore {
    values: std::cell::RefCell<std::collections::BTreeMap<String, String>>,
    bookmarks: std::cell::RefCell<Vec<frost_protocol::BookmarkRecord>>,
    history: std::cell::RefCell<Vec<frost_protocol::HistoryRecord>>,
    downloads: std::cell::RefCell<Vec<frost_protocol::DownloadRecord>>,
    permissions: std::cell::RefCell<Vec<frost_protocol::PermissionRecord>>,
}

impl SettingsRepository for InMemoryStore {
    fn get_setting(&self, key: &str) -> frost_store::StoreResult<Option<String>> {
        Ok(self.values.borrow().get(key).cloned())
    }

    fn set_setting(&self, key: &str, value: &str) -> frost_store::StoreResult<()> {
        self.values
            .borrow_mut()
            .insert(key.to_owned(), value.to_owned());
        Ok(())
    }
}

impl BookmarkRepository for InMemoryStore {
    fn list_bookmarks(&self) -> frost_store::StoreResult<Vec<frost_protocol::BookmarkRecord>> {
        Ok(self.bookmarks.borrow().clone())
    }

    fn save_bookmark(
        &self,
        title: &str,
        url: &str,
        favicon_url: &str,
    ) -> frost_store::StoreResult<bool> {
        if url.is_empty() || url.starts_with("fubuki://") || url.starts_with("data:") {
            return Ok(false);
        }
        let mut bookmarks = self.bookmarks.borrow_mut();
        bookmarks.retain(|b| b.url != url);
        bookmarks.push(frost_protocol::BookmarkRecord {
            title: if title.is_empty() { url } else { title }.to_owned(),
            url: url.to_owned(),
            favicon_url: favicon_url.to_owned(),
            created_at: "memory".into(),
        });
        Ok(true)
    }

    fn remove_bookmark(&self, url: &str) -> frost_store::StoreResult<bool> {
        let mut bookmarks = self.bookmarks.borrow_mut();
        let before = bookmarks.len();
        bookmarks.retain(|b| b.url != url);
        Ok(bookmarks.len() != before)
    }
}

impl HistoryRepository for InMemoryStore {
    fn list_history(&self) -> frost_store::StoreResult<Vec<frost_protocol::HistoryRecord>> {
        Ok(self.history.borrow().clone())
    }

    fn add_history(
        &self,
        title: &str,
        url: &str,
        favicon_url: &str,
    ) -> frost_store::StoreResult<()> {
        let mut history = self.history.borrow_mut();
        history.retain(|record| record.url != url);
        history.push(frost_protocol::HistoryRecord {
            title: if title.is_empty() { url } else { title }.to_owned(),
            url: url.to_owned(),
            favicon_url: favicon_url.to_owned(),
            created_at: "memory".into(),
        });
        Ok(())
    }

    fn remove_history(&self, url: &str) -> frost_store::StoreResult<bool> {
        let mut history = self.history.borrow_mut();
        let before = history.len();
        history.retain(|r| r.url != url);
        Ok(history.len() != before)
    }

    fn clear_history_range(&self, range: &str) -> frost_store::StoreResult<bool> {
        let cutoff = match range {
            "all" => {
                self.history.borrow_mut().clear();
                return Ok(true);
            }
            "lastHour" => now_epoch() - 3600,
            "today" => {
                let now = now_epoch();
                now - (now % 86400)
            }
            _ => return Ok(false),
        };
        let mut history = self.history.borrow_mut();
        let before = history.len();
        // Remove items with timestamps >= cutoff.
        // Items with invalid (non-numeric) timestamps are also removed.
        history.retain(|r| {
            r.created_at
                .parse::<i64>()
                .map(|ts| ts < cutoff)
                .unwrap_or(false)
        });
        Ok(history.len() != before)
    }
}

impl DownloadRepository for InMemoryStore {
    fn list_downloads(&self) -> frost_store::StoreResult<Vec<frost_protocol::DownloadRecord>> {
        Ok(self.downloads.borrow().clone())
    }

    fn upsert_download(
        &self,
        url: &str,
        path: &str,
        state: &str,
        percent: i64,
    ) -> frost_store::StoreResult<()> {
        let mut downloads = self.downloads.borrow_mut();
        downloads.retain(|d| d.url != url || d.path != path);
        downloads.push(frost_protocol::DownloadRecord {
            url: url.to_owned(),
            path: path.to_owned(),
            state: state.to_owned(),
            percent,
            created_at: "memory".into(),
        });
        Ok(())
    }

    fn remove_download(
        &self,
        url: Option<&str>,
        path: Option<&str>,
    ) -> frost_store::StoreResult<bool> {
        let mut downloads = self.downloads.borrow_mut();
        let before = downloads.len();
        downloads.retain(|d| {
            let url_matches = url.is_some_and(|v| !v.is_empty() && v == d.url);
            let path_matches = path.is_some_and(|v| !v.is_empty() && v == d.path);
            !(url_matches || path_matches)
        });
        Ok(downloads.len() != before)
    }
}

impl PermissionRepository for InMemoryStore {
    fn list_permissions(&self) -> frost_store::StoreResult<Vec<frost_protocol::PermissionRecord>> {
        Ok(self.permissions.borrow().clone())
    }

    fn set_permission(
        &self,
        origin: &str,
        permission: &str,
        value: &str,
    ) -> frost_store::StoreResult<()> {
        let mut permissions = self.permissions.borrow_mut();
        permissions.retain(|p| !(p.origin == origin && p.permission == permission));
        permissions.push(frost_protocol::PermissionRecord {
            origin: origin.to_owned(),
            permission: permission.to_owned(),
            value: value.to_owned(),
            created_at: "memory".into(),
        });
        Ok(())
    }

    fn remove_permission(&self, origin: &str, permission: &str) -> frost_store::StoreResult<bool> {
        let mut permissions = self.permissions.borrow_mut();
        let before = permissions.len();
        permissions.retain(|p| !(p.origin == origin && p.permission == permission));
        Ok(permissions.len() != before)
    }
}

impl frost_store::LogRepository for InMemoryStore {
    fn add_log(&self, _level: &str, _message: &str) -> frost_store::StoreResult<()> {
        Ok(())
    }

    fn list_logs(&self, _limit: usize) -> frost_store::StoreResult<Vec<frost_store::LogRecord>> {
        Ok(Vec::new())
    }

    fn clear_logs(&self) -> frost_store::StoreResult<()> {
        Ok(())
    }
}

impl frost_store::SessionRepository for InMemoryStore {
    fn get_session(&self) -> frost_store::StoreResult<Option<String>> {
        Ok(None)
    }

    fn set_session(&self, _json: &str) -> frost_store::StoreResult<()> {
        Ok(())
    }
}

impl frost_store::ClearRepository for InMemoryStore {
    fn clear_bookmarks(&self) -> frost_store::StoreResult<()> {
        self.bookmarks.borrow_mut().clear();
        Ok(())
    }

    fn clear_downloads(&self) -> frost_store::StoreResult<()> {
        self.downloads.borrow_mut().clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use frost_protocol::{HostCommand, HostEvent, HostEventEnvelope, Request, Response};

    use super::*;

    #[test]
    fn creates_and_activates_tabs() {
        let mut core = BrowserCore::new();

        let response = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
            window_id: None,
        }));

        assert!(response.ok);
        assert_eq!(response.response, Response::Bool(true));

        let snapshot = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(state) = snapshot.response else {
            panic!("expected snapshot");
        };
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.tabs[0].url, "https://example.com");
        assert!(state.tabs[0].is_active);
        assert_eq!(state.windows[0].tab_ids, vec![state.tabs[0].id.clone()]);
    }

    #[test]
    fn emits_differential_events() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::new();
        core.set_event_sender(tx);

        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: None,
            active: true,
            window_id: None,
        }));

        let event = rx.try_recv().unwrap();
        assert!(matches!(event.event, Event::TabCreated(_)));
    }

    #[test]
    fn emits_host_command_for_host_side_effects() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );

        let response = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
            window_id: None,
        }));

        let Response::Operation(operation) = response.response else {
            panic!("queued host dispatch must return an operation acknowledgement");
        };
        assert!(operation.pending);
        assert_eq!(operation.status, frost_protocol::OperationStatus::Pending);
        let command = host_rx.try_recv().unwrap();
        assert_eq!(operation.operation_id, command.operation_id);
        assert!(matches!(
            &command.command,
            HostCommand::PageCreate { url, .. } if url == "https://example.com"
        ));
        let snapshot = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(state) = snapshot.response else {
            panic!("expected snapshot");
        };
        assert!(state.tabs.is_empty());
        core.process_host_command_result(HostCommandResultEnvelope::success(command.operation_id))
            .unwrap();
        let snapshot = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(state) = snapshot.response else {
            panic!("expected snapshot");
        };
        assert_eq!(state.tabs.len(), 1);
    }

    #[test]
    fn commands_execute_preserves_pending_host_operation_at_top_level() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );
        let response = core.process(ProtocolRequest::new(Request::CommandsExecute {
            id: "tabs.create".into(),
            args: None,
        }));
        let Response::Operation(operation) = response.response else {
            panic!("commands.execute must not nest a pending operation");
        };
        assert_eq!(
            operation.operation_id,
            host_rx.try_recv().unwrap().operation_id
        );
    }

    #[test]
    fn hundred_host_confirmed_tab_cycles_keep_rust_ids_in_sync() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );
        let mut issued_tab_ids = HashSet::new();
        let mut current_tab_id: Option<String> = None;

        for index in 0..100 {
            let response = core.process(ProtocolRequest::new(Request::TabsCreate {
                url: Some(format!("https://example.test/{index}")),
                active: true,
                window_id: None,
            }));
            let Response::Operation(operation) = response.response else {
                panic!("tab creation {index} must wait for host completion");
            };
            let create = host_rx.try_recv().unwrap();
            let HostCommand::PageCreate { tab_id, .. } = &create.command else {
                panic!("expected page.create");
            };
            assert!(issued_tab_ids.insert(tab_id.clone()));
            assert_eq!(operation.operation_id, create.operation_id);
            core.process_host_command_result(HostCommandResultEnvelope::success(
                create.operation_id,
            ))
            .unwrap();

            if let Some(previous_tab_id) = current_tab_id.take() {
                let close = core.process(ProtocolRequest::new(Request::TabsClose {
                    tab_id: previous_tab_id.clone(),
                }));
                let Response::Operation(operation) = close.response else {
                    panic!("tab close {index} must wait for host completion");
                };
                let command = host_rx.try_recv().unwrap();
                assert!(matches!(
                    &command.command,
                    HostCommand::PageClose { tab_id } if tab_id == &previous_tab_id
                ));
                assert_eq!(operation.operation_id, command.operation_id);
                core.process_host_command_result(HostCommandResultEnvelope::success(
                    command.operation_id,
                ))
                .unwrap();
            }

            let snapshot = core.snapshot().unwrap();
            assert_eq!(snapshot.tabs.len(), 1);
            assert_eq!(snapshot.tabs[0].id, *tab_id);
            assert_eq!(snapshot.windows[0].tab_ids, vec![tab_id.clone()]);
            current_tab_id = Some(tab_id.clone());
        }
    }

    #[test]
    fn records_each_queued_operation_once_at_its_terminal_result() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let (event_tx, event_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );
        core.set_event_sender(event_tx);

        let response = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
            window_id: None,
        }));
        let Response::Operation(operation) = response.response else {
            panic!("expected a pending operation response");
        };
        let command = host_rx.try_recv().unwrap();
        assert_eq!(operation.operation_id, command.operation_id);

        let error = core
            .process_host_command_result(HostCommandResultEnvelope::failure(
                command.operation_id.clone(),
                "host rejected page creation",
            ))
            .unwrap_err();
        assert!(error.to_string().contains("host rejected page creation"));
        assert!(matches!(
            event_rx.try_recv().unwrap().event,
            Event::HostOperationCompleted(frost_protocol::OperationCompleted {
                operation_id,
                status: frost_protocol::OperationCompletionStatus::Failed,
                error: Some(message),
            }) if operation_id == command.operation_id && message == "host rejected page creation"
        ));
        assert!(
            core.process_host_command_result(HostCommandResultEnvelope::success(
                command.operation_id
            ))
            .is_err()
        );
        assert!(event_rx.try_recv().is_err());
    }

    #[test]
    fn private_window_request_queues_an_isolated_runtime_without_persistent_state() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );
        let before = core.snapshot().unwrap();

        let response = core.process(ProtocolRequest::new(Request::WindowsCreatePrivate));
        let Response::Operation(operation) = response.response else {
            panic!("private runtime creation must be pending when host-backed");
        };
        let command = host_rx.try_recv().unwrap();
        assert_eq!(operation.operation_id, command.operation_id);
        assert_eq!(command.command, HostCommand::RuntimeCreatePrivate);

        let during = core.snapshot().unwrap();
        assert_eq!(during.windows, before.windows);
        assert!(during.windows.iter().all(|window| !window.is_private));

        core.process_host_command_result(HostCommandResultEnvelope::success(command.operation_id))
            .unwrap();
        let after = core.snapshot().unwrap();
        assert_eq!(after.windows, before.windows);
        assert!(after.windows.iter().all(|window| !window.is_private));
    }

    #[test]
    fn tick_expires_operations_without_bridge_traffic() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let (event_tx, event_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );
        core.set_event_sender(event_tx);

        let response = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
            window_id: None,
        }));
        let Response::Operation(operation) = response.response else {
            panic!("expected a pending operation response");
        };
        let command = host_rx.try_recv().unwrap();

        core.expire_pending_operations_at(Instant::now() + HOST_OPERATION_TIMEOUT);
        assert!(matches!(
            event_rx.try_recv().unwrap().event,
            Event::HostOperationCompleted(frost_protocol::OperationCompleted {
                operation_id,
                status: frost_protocol::OperationCompletionStatus::TimedOut,
                error: Some(message),
            }) if operation_id == operation.operation_id && message == "host operation timed out"
        ));
        assert!(
            core.process_host_command_result(HostCommandResultEnvelope::success(
                command.operation_id
            ))
            .is_err()
        );
        assert!(event_rx.try_recv().is_err());
    }

    #[test]
    fn applies_host_page_events_to_core_state() {
        let mut core = BrowserCore::new();
        let create = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(create.ok);
        let snapshot = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(state) = snapshot.response else {
            panic!("expected snapshot");
        };
        let tab_id = state.tabs[0].id.clone();

        core.process_host_event(HostEventEnvelope::new(HostEvent::PageTitleChanged {
            tab_id: tab_id.clone(),
            title: "Example Domain".into(),
        }))
        .unwrap();

        let snapshot = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(state) = snapshot.response else {
            panic!("expected snapshot");
        };
        assert_eq!(state.tabs[0].title, "Example Domain");
        assert!(matches!(
            core.recent_events().last().map(|event| &event.event),
            Some(Event::TabUpdated(TabPatch { tab_id: updated, title: Some(title), .. }))
                if updated == &tab_id && title == "Example Domain"
        ));

        core.process_host_event(HostEventEnvelope::new(HostEvent::PageNavigationFinished {
            tab_id: tab_id.clone(),
        }))
        .unwrap();
        let snapshot = core.snapshot().unwrap();
        assert_eq!(snapshot.history.len(), 1);
        assert_eq!(snapshot.history[0].title, "Example Domain");
        assert_eq!(snapshot.history[0].url, "https://example.com");
    }

    #[test]
    fn unknown_host_result_is_rejected() {
        let mut core = BrowserCore::new();
        let result = HostCommandResultEnvelope::success("cmd-1");
        assert!(core.process_host_command_result(result).is_err());
    }

    #[test]
    fn host_command_result_error_is_core_error() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );
        core.process(ProtocolRequest::new(Request::WindowsCreate));
        let operation_id = host_rx.try_recv().unwrap().operation_id;
        let result = HostCommandResultEnvelope::failure(operation_id.clone(), "host blew up");
        let err = core.process_host_command_result(result).unwrap_err();
        assert!(err.to_string().contains(&operation_id));
        assert!(err.to_string().contains("host blew up"));
    }

    #[test]
    fn host_confirmed_new_window_queues_an_initial_page() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );

        let response = core.process(ProtocolRequest::new(Request::WindowsCreate));
        let Response::Operation(window_operation) = response.response else {
            panic!("window creation must wait for its host result");
        };
        let window_command = host_rx.try_recv().unwrap();
        let HostCommand::WindowCreate { window_id, .. } = &window_command.command else {
            panic!("expected window.create");
        };
        assert_eq!(window_operation.operation_id, window_command.operation_id);
        let window_id = window_id.clone();

        core.process_host_command_result(HostCommandResultEnvelope::success(
            window_command.operation_id,
        ))
        .unwrap();

        let page_command = host_rx.try_recv().unwrap();
        assert!(matches!(
            &page_command.command,
            HostCommand::PageCreate {
                window_id: target_window,
                url,
                ..
            } if target_window == &window_id && url == "fubuki://newtab/"
        ));
        core.process_host_command_result(HostCommandResultEnvelope::success(
            page_command.operation_id,
        ))
        .unwrap();

        let state = core.snapshot().unwrap();
        let window = state
            .windows
            .iter()
            .find(|window| window.id == window_id)
            .unwrap();
        assert_eq!(window.tab_ids.len(), 1);
        assert_eq!(window.active_tab_id.as_ref(), window.tab_ids.first());
        assert!(state.tabs.iter().any(|tab| {
            tab.id == window.tab_ids[0] && tab.window_id == window_id && tab.is_active
        }));
    }

    #[test]
    fn persists_settings_through_repository() {
        let mut core = BrowserCore::new();
        let set = core.process(ProtocolRequest::new(Request::SettingsSet {
            key: "theme".into(),
            value: "dark".into(),
        }));
        assert_eq!(set.response, Response::Bool(true));

        let get = core.process(ProtocolRequest::new(Request::SettingsGet {
            key: "theme".into(),
        }));
        assert_eq!(get.response, Response::Setting(Some("dark".into())));
    }

    #[test]
    fn manages_bookmarks_through_protocol() {
        let mut core = BrowserCore::new();
        let save = core.process(ProtocolRequest::new(Request::BookmarksSave {
            title: "Example".into(),
            url: "https://example.com".into(),
            favicon_url: Some(String::new()),
        }));
        assert_eq!(save.response, Response::Bool(true));

        let list = core.process(ProtocolRequest::new(Request::BookmarksList));
        let Response::BookmarksList(bookmarks) = list.response else {
            panic!("expected bookmarks");
        };
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].url, "https://example.com");
    }

    // ── Multi-window tests ────────────────────────────────────────────────

    /// Helper: create a window and return (window_id, [tab_ids]).
    fn create_window_with_tabs(core: &mut BrowserCore, count: usize) -> (String, Vec<String>) {
        assert!(count > 0);
        let resp = core.process(ProtocolRequest::new(Request::WindowsCreate));
        assert_eq!(resp.response, Response::Bool(true));
        let snapshot = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(state) = snapshot.response else {
            panic!("expected snapshot");
        };
        let window_id = state.windows.last().unwrap().id.clone();
        let mut tab_ids = state
            .tabs
            .iter()
            .filter(|tab| tab.window_id == window_id)
            .map(|tab| tab.id.clone())
            .collect::<Vec<_>>();
        assert_eq!(tab_ids.len(), 1);
        for i in 1..count {
            let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
                url: Some(format!("https://example{}.com", i)),
                active: true,
                window_id: None,
            }));
            assert!(resp.ok);
            let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
            let Response::AppSnapshot(s) = snap.response else {
                panic!("expected snapshot");
            };
            let new_tab = s.tabs.last().unwrap();
            assert_eq!(new_tab.window_id, window_id);
            tab_ids.push(new_tab.id.clone());
        }
        (window_id, tab_ids)
    }

    #[test]
    fn close_other_tabs_scoped_to_window() {
        let mut core = BrowserCore::new();
        // Window 1 (default) – 1 tab
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://w1.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(resp.ok);
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s1) = snap.response else {
            panic!()
        };
        let w1_tab = s1.tabs.last().unwrap().id.clone();

        // Window 2 – 3 tabs
        let (_w2_id, w2_tabs) = create_window_with_tabs(&mut core, 3);
        let target = w2_tabs[1].clone(); // middle tab

        // close_other_tabs should only affect Window 2.
        let resp = core.process(ProtocolRequest::new(Request::TabsCloseOther {
            tab_id: target.clone(),
        }));
        assert_eq!(resp.response, Response::Bool(true));

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        // Window 1 tab should still exist.
        assert!(s.tabs.iter().any(|t| t.id == w1_tab));
        // Window 2 should have only the target tab + pinned (none pinned here).
        let w2_id = &s.windows.last().unwrap().id;
        let w2_remaining: Vec<_> = s.tabs.iter().filter(|t| t.window_id == *w2_id).collect();
        assert_eq!(w2_remaining.len(), 1);
        assert_eq!(w2_remaining[0].id, target);
    }

    #[test]
    fn close_tabs_to_right_fails_without_host_ordering_contract() {
        let mut core = BrowserCore::new();
        // Window 1 – 1 tab
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://w1.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(resp.ok);
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s1) = snap.response else {
            panic!()
        };
        let w1_tab = s1.tabs.last().unwrap().id.clone();

        // Window 2 – 3 tabs
        let (_w2_id, w2_tabs) = create_window_with_tabs(&mut core, 3);
        let target = w2_tabs[0].clone(); // first tab in window 2

        let resp = core.process(ProtocolRequest::new(Request::TabsCloseToRight {
            tab_id: target.clone(),
        }));
        assert!(!resp.ok);
        assert!(
            matches!(resp.response, Response::Error(message) if message.contains("host-confirmed"))
        );
        assert!(
            core.snapshot()
                .unwrap()
                .tabs
                .iter()
                .any(|tab| tab.id == w1_tab)
        );
    }

    #[test]
    fn move_tab_only_within_same_window() {
        let mut core = BrowserCore::new();
        let (w2_id, w2_tabs) = create_window_with_tabs(&mut core, 3);
        let target = w2_tabs[0].clone();

        // Move to index 2 (should stay in window 2).
        let resp = core.process(ProtocolRequest::new(Request::TabsMove {
            tab_id: target.clone(),
            to_index: 2,
        }));
        assert_eq!(resp.response, Response::Bool(true));

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let w2_tabs_now: Vec<_> = s.tabs.iter().filter(|t| t.window_id == w2_id).collect();
        // The target tab should now be last (index 2).
        assert_eq!(w2_tabs_now.last().unwrap().id, target);
        // Total tab count unchanged.
        assert_eq!(s.tabs.len(), 3); // default window has no tabs + 3 w2
    }

    #[test]
    fn move_tab_to_window_fails_without_atomic_host_reparenting() {
        let mut core = BrowserCore::new();
        let (_, w2_tabs) = create_window_with_tabs(&mut core, 2);

        // Move a tab from w2 into a new window via TabsMoveToNewWindow.
        let resp = core.process(ProtocolRequest::new(Request::TabsMoveToNewWindow {
            tab_id: w2_tabs[0].clone(),
        }));
        assert!(!resp.ok);
        assert!(
            core.snapshot()
                .unwrap()
                .tabs
                .iter()
                .any(|tab| tab.id == w2_tabs[0])
        );
    }

    #[test]
    fn reopening_window_fails_without_atomic_host_restore() {
        let mut core = BrowserCore::new();
        let (w2_id, _w2_tabs) = create_window_with_tabs(&mut core, 2);

        // Close window 2.
        let resp = core.process(ProtocolRequest::new(Request::WindowsClose {
            window_id: Some(w2_id.clone()),
        }));
        assert_eq!(resp.response, Response::Bool(true));

        // Window 2's tabs should be gone from core.
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        assert!(!s.windows.iter().any(|w| w.id == w2_id));
        assert!(!s.tabs.iter().any(|t| t.window_id == w2_id));

        // Reopening must not recreate Rust state until the host can restore
        // every page as one operation.
        let resp = core.process(ProtocolRequest::new(Request::WindowsReopenClosed));
        assert!(!resp.ok);
        assert!(
            !core
                .snapshot()
                .unwrap()
                .windows
                .iter()
                .any(|window| window.id == w2_id)
        );
    }

    #[test]
    fn only_one_active_tab_per_window() {
        let mut core = BrowserCore::new();
        let (_, w2_tabs) = create_window_with_tabs(&mut core, 3);

        // Activate each tab in sequence and verify only one is active.
        for tab_id in &w2_tabs {
            let resp = core.process(ProtocolRequest::new(Request::TabsActivate {
                tab_id: tab_id.clone(),
            }));
            assert!(resp.ok);
        }

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        // Only one active tab total (all in same window, default window has no tabs).
        let active_count = s.tabs.iter().filter(|t| t.is_active).count();
        assert_eq!(active_count, 1, "only one tab should be active");
    }
}
