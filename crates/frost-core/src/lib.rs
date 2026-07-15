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
    time::{SystemTime, UNIX_EPOCH},
};

use crossbeam_channel::{Receiver, Sender};
use frost_engine_api::{EngineAdapter, EngineError, EngineResult, NoopEngineAdapter};
use frost_protocol::{
    BrowserCommand, Event, EventEnvelope, HostCommand, HostCommandEnvelope,
    HostCommandResultEnvelope, HostEvent, HostEventEnvelope, ProtocolRequest, ProtocolResponse,
    Request, Response, SettingChanged, TabActivated, TabClosed, TabMoved, TabPatch,
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

/// Tracks a pending operation that can be rolled back on host command failure.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum PendingOperation {
    TabCreated {
        tab_id: String,
        window_id: String,
    },
    TabMoved {
        tab_id: String,
        from_window_id: String,
        to_window_id: String,
    },
    TabActivated {
        tab_id: String,
        previous_active_tab_id: Option<String>,
    },
    WindowCreated {
        window_id: String,
    },
}

/// Snapshot of state before an operation for rollback purposes.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct StateSnapshot {
    tabs: Vec<frost_protocol::TabState>,
    windows: Vec<frost_protocol::WindowState>,
    active_window_id: Option<String>,
}

pub struct HostCommandAdapter {
    tx: Sender<HostCommandEnvelope>,
    last_command_id: Option<String>,
}

impl HostCommandAdapter {
    pub fn new(tx: Sender<HostCommandEnvelope>) -> Self {
        Self {
            tx,
            last_command_id: None,
        }
    }

    fn send(&mut self, command: HostCommand) -> EngineResult<()> {
        let id = format!("host-command-{}", uuid::Uuid::new_v4());
        self.last_command_id = Some(id.clone());
        self.tx
            .send(HostCommandEnvelope::new(id, command))
            .map_err(|e| EngineError::Message(e.to_string()))
    }
}

impl EngineAdapter for HostCommandAdapter {
    fn last_command_id(&self) -> Option<&str> {
        self.last_command_id.as_deref()
    }

    fn create_page(
        &mut self,
        tab_id: &str,
        window_id: &str,
        url: &str,
        active: bool,
    ) -> EngineResult<()> {
        self.send(HostCommand::PageCreate {
            tab_id: tab_id.to_owned(),
            window_id: window_id.to_owned(),
            url: url.to_owned(),
            active,
        })
    }

    fn close_page(&mut self, tab_id: &str) -> EngineResult<()> {
        self.send(HostCommand::PageClose {
            tab_id: tab_id.to_owned(),
        })
    }

    fn activate_page(&mut self, tab_id: &str) -> EngineResult<()> {
        self.send(HostCommand::PageActivate {
            tab_id: tab_id.to_owned(),
        })
    }

    fn pin_page(&mut self, tab_id: &str, pinned: bool) -> EngineResult<()> {
        self.send(HostCommand::PagePin {
            tab_id: tab_id.to_owned(),
            pinned,
        })
    }

    fn move_page(&mut self, tab_id: &str, to_index: usize) -> EngineResult<()> {
        self.send(HostCommand::PageMove {
            tab_id: tab_id.to_owned(),
            to_index,
        })
    }

    fn move_page_to_window(&mut self, tab_id: &str, window_id: &str) -> EngineResult<()> {
        self.send(HostCommand::PageMoveToWindow {
            tab_id: tab_id.to_owned(),
            window_id: window_id.to_owned(),
        })
    }

    fn navigate(&mut self, tab_id: &str, input: &str) -> EngineResult<()> {
        self.send(HostCommand::PageNavigate {
            tab_id: tab_id.to_owned(),
            url: input.to_owned(),
        })
    }

    fn reload(&mut self, tab_id: &str) -> EngineResult<()> {
        self.send(HostCommand::PageReload {
            tab_id: tab_id.to_owned(),
        })
    }

    fn stop(&mut self, tab_id: &str) -> EngineResult<()> {
        self.send(HostCommand::PageStop {
            tab_id: tab_id.to_owned(),
        })
    }

    fn go_back(&mut self, tab_id: &str) -> EngineResult<()> {
        self.send(HostCommand::PageGoBack {
            tab_id: tab_id.to_owned(),
        })
    }

    fn go_forward(&mut self, tab_id: &str) -> EngineResult<()> {
        self.send(HostCommand::PageGoForward {
            tab_id: tab_id.to_owned(),
        })
    }

    fn create_window(&mut self, window_id: &str, is_private: bool) -> EngineResult<()> {
        self.send(HostCommand::WindowCreate {
            window_id: window_id.to_owned(),
            is_private,
        })
    }

    fn close_window(&mut self, window_id: &str) -> EngineResult<()> {
        self.send(HostCommand::WindowClose {
            window_id: window_id.to_owned(),
        })
    }
}

pub struct BrowserCore<A = NoopEngineAdapter, S = InMemoryStore> {
    adapter: A,
    repository: S,
    tabs: TabService,
    windows: WindowService,
    closed_tabs: Vec<frost_protocol::TabState>,
    closed_windows: Vec<ClosedWindow>,
    events: Vec<EventEnvelope>,
    event_tx: Option<Sender<EventEnvelope>>,
    /// Tracks pending operations for rollback on host command failure
    pending_operations: HashMap<String, PendingOperation>,
}

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
        let mut windows = WindowService::new();
        let window_id = windows.create_window(false);
        let tabs = TabService::new(window_id);

        Self {
            adapter,
            repository,
            tabs,
            windows,
            closed_tabs: Vec::new(),
            closed_windows: Vec::new(),
            events: Vec::new(),
            event_tx: None,
            pending_operations: HashMap::new(),
        }
    }

    pub fn set_event_sender(&mut self, sender: Sender<EventEnvelope>) {
        self.event_tx = Some(sender);
    }

    pub fn recent_events(&self) -> &[EventEnvelope] {
        &self.events
    }

    pub fn process(&mut self, request: ProtocolRequest) -> ProtocolResponse {
        let id = request.id.clone();
        match self.process_inner(request.request) {
            Ok(response) => ProtocolResponse::ok(id, response),
            Err(error) => ProtocolResponse::error(id, error.to_string()),
        }
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
            Request::AppSnapshot => Ok(Response::AppSnapshot(self.snapshot())),
            Request::TabsList => Ok(Response::TabsList(self.tabs.list())),
            Request::TabsCreate {
                url,
                active,
                window_id,
            } => {
                let window_id = window_id
                    .or_else(|| self.windows.active_window_id().map(ToOwned::to_owned))
                    .ok_or_else(|| CoreError::Message("No active window".into()))?;
                // Validate that the target window exists
                if self.windows.get_window(&window_id).is_none() {
                    return Err(CoreError::Message(format!(
                        "Window '{}' does not exist",
                        window_id
                    )));
                }
                let tab = self.tabs.create_tab(
                    window_id.clone(),
                    url.unwrap_or_else(|| "fubuki://newtab/".into()),
                    active,
                );
                // Only update WindowState.active_tab_id when creating an active tab
                self.windows.attach_tab(&window_id, &tab.id, active);
                if let Err(e) = self
                    .adapter
                    .create_page(&tab.id, &window_id, &tab.url, active)
                {
                    self.windows.detach_tab(&tab.id);
                    self.tabs.remove_tab(&tab.id);
                    return Err(CoreError::Message(e.to_string()));
                }
                self.record_pending(PendingOperation::TabCreated {
                    tab_id: tab.id.clone(),
                    window_id: window_id.clone(),
                });
                self.emit(Event::TabCreated(tab));
                Ok(Response::Bool(true))
            }
            Request::TabsActivate { tab_id } => {
                let activated = self.tabs.activate_tab(&tab_id);
                if activated {
                    self.windows.set_active_tab(&tab_id);
                    self.adapter
                        .activate_page(&tab_id)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::TabActivated(TabActivated { tab_id }));
                }
                Ok(Response::Bool(activated))
            }
            Request::TabsClose { tab_id } => {
                // Capture the tab state before closing
                let was_active = self
                    .tabs
                    .get_tab(&tab_id)
                    .map(|t| t.is_active)
                    .unwrap_or(false);
                let window_id = self.tabs.get_tab(&tab_id).map(|t| t.window_id.clone());
                if let Some(tab) = self.tabs.get_tab(&tab_id) {
                    self.closed_tabs.push(tab);
                    if self.closed_tabs.len() > 50 {
                        self.closed_tabs.remove(0);
                    }
                }
                let closed = self.tabs.close_tab(&tab_id);
                if closed {
                    self.windows.detach_tab(&tab_id);
                    self.adapter
                        .close_page(&tab_id)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::TabClosed(TabClosed {
                        tab_id: tab_id.clone(),
                    }));
                    // If the closed tab was active, emit tab.activated for the newly selected tab
                    if was_active
                        && let Some(ref wid) = window_id
                        && let Some(new_active) =
                            self.tabs.tabs_in_window(wid).iter().find(|t| t.is_active)
                    {
                        self.emit(Event::TabActivated(TabActivated {
                            tab_id: new_active.id.clone(),
                        }));
                    }
                    if let Some(wid) = window_id
                        && self.tabs.tabs_in_window(&wid).is_empty()
                    {
                        let close_window =
                            SettingsService::get(&self.repository, "closeWindowWithLastTab")
                                .ok()
                                .flatten()
                                .is_some_and(|value| value == "true");
                        if close_window {
                            self.windows.close_window(&wid);
                            self.adapter
                                .close_window(&wid)
                                .map_err(|e| CoreError::Message(e.to_string()))?;
                            self.emit(Event::WindowClosed { window_id: wid });
                        } else {
                            let empty =
                                self.tabs
                                    .create_tab(wid.clone(), "fubuki://newtab/".into(), true);
                            self.windows.attach_tab(&wid, &empty.id, true);
                            self.adapter
                                .create_page(&empty.id, &wid, &empty.url, true)
                                .map_err(|e| CoreError::Message(e.to_string()))?;
                            self.emit(Event::TabCreated(empty));
                        }
                    }
                }
                Ok(Response::Bool(closed))
            }
            Request::TabsPin { tab_id, pinned } => {
                let ok = self.tabs.pin_tab(&tab_id, pinned);
                if ok {
                    self.adapter
                        .pin_page(&tab_id, pinned)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        is_pinned: Some(pinned),
                        ..Default::default()
                    }));
                }
                Ok(Response::Bool(ok))
            }
            Request::TabsDuplicate { tab_id } => {
                let Some(tab) = self.tabs.duplicate_tab(&tab_id) else {
                    return Ok(Response::Bool(false));
                };
                // Duplicate always creates a background tab
                self.windows.attach_tab(&tab.window_id, &tab.id, false);
                if let Err(e) = self
                    .adapter
                    .create_page(&tab.id, &tab.window_id, &tab.url, false)
                {
                    self.windows.detach_tab(&tab.id);
                    self.tabs.remove_tab(&tab.id);
                    return Err(CoreError::Message(e.to_string()));
                }
                self.emit(Event::TabCreated(tab));
                Ok(Response::Bool(true))
            }
            Request::TabsReopenClosed => {
                let Some(mut tab) = self.closed_tabs.pop() else {
                    return Ok(Response::Bool(false));
                };
                tab.is_active = true;
                // Ensure the original window still exists; fall back to active window
                let target_window = if self.windows.get_window(&tab.window_id).is_some() {
                    tab.window_id.clone()
                } else if let Some(active) = self.windows.active_window_id() {
                    active.to_owned()
                } else {
                    return Ok(Response::Bool(false));
                };
                tab.window_id = target_window.clone();
                let created = tab.clone();
                self.tabs.upsert_tab(tab);
                self.windows.attach_tab(&target_window, &created.id, true);
                if let Err(e) =
                    self.adapter
                        .create_page(&created.id, &target_window, &created.url, true)
                {
                    self.windows.detach_tab(&created.id);
                    self.tabs.remove_tab(&created.id);
                    return Err(CoreError::Message(e.to_string()));
                }
                self.emit(Event::TabCreated(created));
                Ok(Response::Bool(true))
            }
            Request::TabsCloseOther { tab_id } => {
                let was_active = self
                    .tabs
                    .get_tab(&tab_id)
                    .map(|tab| tab.is_active)
                    .unwrap_or(false);
                let closed = self.tabs.close_other_tabs(&tab_id);
                for tab in &closed {
                    self.windows.detach_tab(&tab.id);
                    self.adapter
                        .close_page(&tab.id)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::TabClosed(TabClosed {
                        tab_id: tab.id.clone(),
                    }));
                }
                // Sync WindowState.active_tab_id with the kept tab
                self.tabs.activate_tab(&tab_id);
                self.windows.set_active_tab(&tab_id);
                if !was_active {
                    self.emit(Event::TabActivated(TabActivated {
                        tab_id: tab_id.clone(),
                    }));
                }
                self.closed_tabs.extend(closed);
                self.trim_closed_tabs();
                Ok(Response::Bool(true))
            }
            Request::TabsCloseToRight { tab_id } => {
                let was_active = self
                    .tabs
                    .get_tab(&tab_id)
                    .map(|tab| tab.is_active)
                    .unwrap_or(false);
                let closed = self.tabs.close_tabs_to_right(&tab_id);
                for tab in &closed {
                    self.windows.detach_tab(&tab.id);
                    self.adapter
                        .close_page(&tab.id)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::TabClosed(TabClosed {
                        tab_id: tab.id.clone(),
                    }));
                }
                // Ensure a non-empty window still has an active tab
                if self.tabs.get_tab(&tab_id).is_some() {
                    self.tabs.activate_tab(&tab_id);
                    self.windows.set_active_tab(&tab_id);
                    if !was_active {
                        self.emit(Event::TabActivated(TabActivated {
                            tab_id: tab_id.clone(),
                        }));
                    }
                }
                self.closed_tabs.extend(closed);
                self.trim_closed_tabs();
                Ok(Response::Bool(true))
            }
            Request::TabsMove { tab_id, to_index } => {
                let window_id = self
                    .tabs
                    .get_tab(&tab_id)
                    .map(|tab| tab.window_id.clone())
                    .unwrap_or_default();
                let ok = self.tabs.move_tab(&tab_id, to_index);
                if ok {
                    // Also update WindowState.tab_ids order
                    self.windows.move_tab_in_window(&tab_id, to_index);
                    self.adapter
                        .move_page(&tab_id, to_index)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::TabMoved(TabMoved {
                        tab_id,
                        from_window_id: window_id.clone(),
                        to_window_id: window_id,
                        to_index,
                    }));
                }
                Ok(Response::Bool(ok))
            }
            Request::TabsMoveToNewWindow { tab_id } => {
                if !self.tabs.contains(&tab_id) {
                    return Ok(Response::Bool(false));
                }
                let previous_window = self.tabs.get_tab(&tab_id).map(|tab| tab.window_id.clone());
                // Preserve private flag from the source window
                let is_private = previous_window
                    .as_ref()
                    .and_then(|wid| self.windows.get_window(wid))
                    .map(|w| w.is_private)
                    .unwrap_or(false);
                let new_window_id = self.windows.create_window(is_private);
                self.tabs.move_tab_to_window(&tab_id, &new_window_id);
                self.windows.move_tab_to_window(&tab_id, &new_window_id);
                if let Err(e) = self.adapter.move_page_to_window(&tab_id, &new_window_id) {
                    self.tabs
                        .move_tab_to_window(&tab_id, &previous_window.clone().unwrap_or_default());
                    self.windows
                        .move_tab_to_window(&tab_id, &previous_window.clone().unwrap_or_default());
                    self.windows.close_window(&new_window_id);
                    return Err(CoreError::Message(e.to_string()));
                }
                self.record_pending(PendingOperation::TabMoved {
                    tab_id: tab_id.clone(),
                    from_window_id: previous_window.clone().unwrap_or_default(),
                    to_window_id: new_window_id.clone(),
                });
                // If the source window is now empty, create an active empty tab there
                if let Some(ref prev_window_id) = previous_window
                    && self.tabs.tabs_in_window(prev_window_id).is_empty()
                {
                    let empty_tab = self.tabs.create_tab(
                        prev_window_id.clone(),
                        "fubuki://newtab/".into(),
                        true,
                    );
                    self.windows.attach_tab(prev_window_id, &empty_tab.id, true);
                    if let Err(e) = self.adapter.create_page(
                        &empty_tab.id,
                        prev_window_id,
                        &empty_tab.url,
                        true,
                    ) {
                        self.windows.detach_tab(&empty_tab.id);
                        self.tabs.remove_tab(&empty_tab.id);
                        return Err(CoreError::Message(e.to_string()));
                    }
                    self.emit(Event::TabCreated(empty_tab));
                }
                if let Some(window) = self.windows.get_window(&new_window_id) {
                    self.emit(Event::WindowCreated(window));
                }
                self.emit(Event::TabUpdated(TabPatch {
                    tab_id,
                    ..Default::default()
                }));
                Ok(Response::Bool(true))
            }
            Request::TabsNavigate { tab_id, input } => {
                let changed = self.tabs.navigate(&tab_id, &input);
                if changed {
                    self.adapter
                        .navigate(&tab_id, &input)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        url: Some(input),
                        is_loading: Some(true),
                        ..Default::default()
                    }));
                }
                Ok(Response::Bool(changed))
            }
            Request::TabsReload { tab_id } => self.host_tab_action(&tab_id, HostTabAction::Reload),
            Request::TabsStop { tab_id } => {
                let ok = self.tabs.stop_tab(&tab_id);
                if ok {
                    self.adapter
                        .stop(&tab_id)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        is_loading: Some(false),
                        ..Default::default()
                    }));
                }
                Ok(Response::Bool(ok))
            }
            Request::TabsGoBack { tab_id } => self.host_tab_action(&tab_id, HostTabAction::GoBack),
            Request::TabsGoForward { tab_id } => {
                self.host_tab_action(&tab_id, HostTabAction::GoForward)
            }
            Request::TabsHome { tab_id, window_id } => {
                let tab = if let Some(tab_id) = tab_id {
                    self.tabs.get_tab(&tab_id).filter(|tab| {
                        window_id
                            .as_deref()
                            .is_none_or(|window| tab.window_id == window)
                    })
                } else if let Some(window_id) = window_id {
                    self.tabs
                        .tabs_in_window(&window_id)
                        .into_iter()
                        .find(|tab| tab.is_active)
                } else {
                    self.tabs.active_tab()
                };
                let Some(tab) = tab else {
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
                let window_id = self.windows.create_window(false);
                if let Err(e) = self.adapter.create_window(&window_id, false) {
                    self.windows.close_window(&window_id);
                    return Err(CoreError::Message(e.to_string()));
                }
                if let Some(window) = self.windows.get_window(&window_id) {
                    self.emit(Event::WindowCreated(window));
                }
                Ok(Response::Bool(true))
            }
            Request::WindowsCreatePrivate => {
                let window_id = self.windows.create_window(true);
                if let Err(e) = self.adapter.create_window(&window_id, true) {
                    self.windows.close_window(&window_id);
                    return Err(CoreError::Message(e.to_string()));
                }
                if let Some(window) = self.windows.get_window(&window_id) {
                    self.emit(Event::WindowCreated(window));
                }
                Ok(Response::Bool(true))
            }
            Request::WindowsClose { window_id } => {
                let target = window_id
                    .or_else(|| self.windows.active_window_id().map(ToOwned::to_owned))
                    .ok_or_else(|| CoreError::Message("No active window".into()))?;
                // Capture full window state with tabs BEFORE closing the window.
                let window_state = self
                    .windows
                    .get_window(&target)
                    .ok_or_else(|| CoreError::Message("Window not found".into()))?;
                let window_tabs: Vec<frost_protocol::TabState> = self
                    .tabs
                    .list()
                    .into_iter()
                    .filter(|t| t.window_id == target)
                    .collect();
                // Now remove the window.
                let closed = self.windows.close_window(&target);
                if closed {
                    self.closed_windows.push(ClosedWindow {
                        window: window_state,
                        tabs: window_tabs.clone(),
                    });
                    if self.closed_windows.len() > 10 {
                        self.closed_windows.remove(0);
                    }
                    // Remove tabs belonging to this window from TabService.
                    for tab in &window_tabs {
                        self.tabs.remove_tab(&tab.id);
                    }
                    self.adapter
                        .close_window(&target)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::WindowClosed { window_id: target });
                }
                Ok(Response::Bool(closed))
            }
            Request::WindowsReopenClosed => {
                let Some(closed) = self.closed_windows.pop() else {
                    return Ok(Response::Bool(false));
                };
                // Restore tabs first.
                for tab in &closed.tabs {
                    self.tabs.upsert_tab(tab.clone());
                }
                // Restore window.
                let mut windows = self.windows.list();
                windows.push(closed.window.clone());
                self.windows
                    .replace_all(windows, Some(closed.window.id.clone()));
                // Emit events for restored tabs.
                for tab in &closed.tabs {
                    self.emit(Event::TabCreated(tab.clone()));
                }
                self.emit(Event::WindowCreated(closed.window));
                Ok(Response::Bool(true))
            }
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
                SettingsService::set(&self.repository, &key, value)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.emit(Event::SettingChanged(SettingChanged {
                    key,
                    value: value.into(),
                }));
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
                    self.emit(Event::DownloadChanged { url, path });
                }
                Ok(Response::Bool(ok))
            }
            Request::DataClear { target } => {
                let target = target.unwrap_or_else(|| "all".into());
                if target == "bookmarks" || target == "all" {
                    let _ = self.repository.clear_bookmarks();
                    self.emit(Event::BookmarkChanged { url: String::new() });
                }
                if target == "history" || target == "all" {
                    let _ = HistoryService::clear_range(&self.repository, "all");
                    self.emit(Event::HistoryChanged { url: None });
                }
                if target == "downloads" || target == "all" {
                    let _ = self.repository.clear_downloads();
                    self.emit(Event::DownloadChanged {
                        url: None,
                        path: None,
                    });
                }
                Ok(Response::Bool(true))
            }
            Request::PermissionsSet {
                origin,
                permission,
                value,
            } => {
                let _ = self.repository.set_permission(&origin, &permission, &value);
                self.emit(Event::PermissionChanged { origin, permission });
                Ok(Response::Bool(true))
            }
            Request::CommandsList => Ok(Response::CommandsList(default_commands())),
            Request::CommandsExecute { id, args: _ } => Ok(Response::Json(serde_json::json!({
                "handled": false,
                "id": id
            }))),
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
            Request::HostSyncSnapshot { state } => {
                let mut errors: Vec<String> = Vec::new();

                for bookmark in &state.bookmarks {
                    if let Err(e) = self.repository.save_bookmark(
                        &bookmark.title,
                        &bookmark.url,
                        &bookmark.favicon_url,
                    ) {
                        errors.push(format!("bookmark '{}': {}", bookmark.url, e));
                    }
                }
                for history in &state.history {
                    if let Err(e) = self.repository.add_history(
                        &history.title,
                        &history.url,
                        &history.favicon_url,
                    ) {
                        errors.push(format!("history '{}': {}", history.url, e));
                    }
                }
                for download in &state.downloads {
                    if let Err(e) = self.repository.upsert_download(
                        &download.url,
                        &download.path,
                        &download.state,
                        download.percent,
                    ) {
                        errors.push(format!("download '{}': {}", download.url, e));
                    }
                }
                for permission in &state.permissions {
                    if let Err(e) = self.repository.set_permission(
                        &permission.origin,
                        &permission.permission,
                        &permission.value,
                    ) {
                        errors.push(format!(
                            "permission '{}/{}': {}",
                            permission.origin, permission.permission, e
                        ));
                    }
                }
                if let Some(settings) = state.settings.as_object() {
                    for (key, value) in settings {
                        if let Some(value) = value.as_str()
                            && let Err(e) = self.repository.set_setting(key, value)
                        {
                            errors.push(format!("setting '{}': {}", key, e));
                        }
                    }
                }
                self.tabs.replace_all(state.tabs);
                self.windows
                    .replace_all(state.windows, state.active_window_id);
                self.emit(Event::HostSynced);
                if errors.is_empty() {
                    Ok(Response::Bool(true))
                } else {
                    Ok(Response::Json(serde_json::json!({
                        "synced": true,
                        "errors": errors,
                    })))
                }
            }
            Request::TabsUnpin { tab_id } => {
                let ok = self.tabs.pin_tab(&tab_id, false);
                if ok {
                    self.adapter
                        .pin_page(&tab_id, false)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::TabUpdated(TabPatch {
                        tab_id,
                        is_pinned: Some(false),
                        ..Default::default()
                    }));
                }
                Ok(Response::Bool(ok))
            }
            Request::TabsActivateNext => {
                let Some(current) = self.tabs.active_tab() else {
                    return Ok(Response::Bool(false));
                };
                let window_id = current.window_id.clone();
                let tabs_in_window = self.tabs.tabs_in_window(&window_id);
                let pos = tabs_in_window.iter().position(|t| t.id == current.id);
                if let Some(pos) = pos {
                    let next = (pos + 1) % tabs_in_window.len();
                    self.process_inner(Request::TabsActivate {
                        tab_id: tabs_in_window[next].id.clone(),
                    })
                } else {
                    Ok(Response::Bool(false))
                }
            }
            Request::TabsActivatePrevious => {
                let Some(current) = self.tabs.active_tab() else {
                    return Ok(Response::Bool(false));
                };
                let window_id = current.window_id.clone();
                let tabs_in_window = self.tabs.tabs_in_window(&window_id);
                let pos = tabs_in_window.iter().position(|t| t.id == current.id);
                if let Some(pos) = pos {
                    let prev = if pos == 0 {
                        tabs_in_window.len() - 1
                    } else {
                        pos - 1
                    };
                    self.process_inner(Request::TabsActivate {
                        tab_id: tabs_in_window[prev].id.clone(),
                    })
                } else {
                    Ok(Response::Bool(false))
                }
            }
            Request::WindowsReopenClosedPrivate => {
                let Some(idx) = self
                    .closed_windows
                    .iter()
                    .rposition(|cw| cw.window.is_private)
                else {
                    return Ok(Response::Bool(false));
                };
                let closed = self.closed_windows.remove(idx);
                for tab in &closed.tabs {
                    self.tabs.upsert_tab(tab.clone());
                }
                let mut windows = self.windows.list();
                windows.push(closed.window.clone());
                self.windows
                    .replace_all(windows, Some(closed.window.id.clone()));
                for tab in &closed.tabs {
                    self.emit(Event::TabCreated(tab.clone()));
                }
                self.emit(Event::WindowCreated(closed.window));
                Ok(Response::Bool(true))
            }
        }
    }

    pub fn process_host_event(&mut self, envelope: HostEventEnvelope) -> CoreResult<()> {
        match envelope.event {
            HostEvent::PageCreated {
                tab_id,
                window_id,
                url,
                active,
                is_private,
            } => {
                self.windows.ensure_window(&window_id, is_private);
                let is_new_tab = self.tabs.get_tab(&tab_id).is_none();
                if let Some(mut tab) = self.tabs.get_tab(&tab_id) {
                    let old_window_id = tab.window_id.clone();
                    tab.window_id = window_id.clone();
                    tab.url = url;
                    tab.is_active = active;
                    self.tabs.upsert_tab(tab);
                    // Detach from old window if it changed
                    if old_window_id != window_id {
                        self.windows.detach_tab(&tab_id);
                    }
                } else {
                    self.tabs.upsert_tab(frost_protocol::TabState {
                        id: tab_id.clone(),
                        window_id: window_id.clone(),
                        title: "New Tab".into(),
                        url,
                        favicon_url: String::new(),
                        error_text: String::new(),
                        zoom_level: 0.0,
                        is_loading: false,
                        can_go_back: false,
                        can_go_forward: false,
                        is_active: active,
                        is_pinned: false,
                    });
                }
                if active {
                    self.tabs.activate_tab(&tab_id);
                }
                self.windows.attach_tab(&window_id, &tab_id, active);
                if is_new_tab && let Some(tab) = self.tabs.get_tab(&tab_id) {
                    self.emit(Event::TabCreated(tab));
                }
                Ok(())
            }
            HostEvent::PageClosed { tab_id } => {
                let closed = self.tabs.close_tab(&tab_id);
                if closed {
                    self.windows.detach_tab(&tab_id);
                    self.emit(Event::TabClosed(TabClosed { tab_id }));
                }
                Ok(())
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
            HostEvent::PageMoved {
                tab_id,
                from_window_id,
                to_window_id,
                to_index,
            } => {
                self.tabs.move_tab_to_window(&tab_id, &to_window_id);
                self.windows.move_tab_to_window(&tab_id, &to_window_id);
                self.tabs.move_tab(&tab_id, to_index);
                // If the source window is now empty, create an active empty tab
                if self.tabs.tabs_in_window(&from_window_id).is_empty() {
                    let empty_tab = self.tabs.create_tab(
                        from_window_id.clone(),
                        "fubuki://newtab/".into(),
                        true,
                    );
                    self.windows
                        .attach_tab(&from_window_id, &empty_tab.id, true);
                    if let Err(e) = self.adapter.create_page(
                        &empty_tab.id,
                        &from_window_id,
                        &empty_tab.url,
                        true,
                    ) {
                        self.windows.detach_tab(&empty_tab.id);
                        self.tabs.remove_tab(&empty_tab.id);
                        return Err(CoreError::Message(e.to_string()));
                    }
                    self.emit(Event::TabCreated(empty_tab));
                }
                Ok(())
            }
            HostEvent::WindowClosed { window_id } => {
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
        if result.ok {
            self.pending_operations.remove(&result.command_id);
            return Ok(());
        }
        // Roll back only the operation identified by this host response.
        if let Some(op) = self.pending_operations.remove(&result.command_id) {
            self.rollback_operation(op);
        }
        Err(CoreError::Message(format!(
            "Host command {} failed: {}",
            result.command_id,
            result.error.unwrap_or_else(|| "unknown error".into())
        )))
    }

    #[allow(dead_code)]
    fn take_snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            tabs: self.tabs.list(),
            windows: self.windows.list(),
            active_window_id: self.windows.active_window_id().map(ToOwned::to_owned),
        }
    }

    #[allow(dead_code)]
    fn restore_from_snapshot(&mut self, snapshot: &StateSnapshot) {
        self.tabs.replace_all(snapshot.tabs.clone());
        self.windows
            .replace_all(snapshot.windows.clone(), snapshot.active_window_id.clone());
    }

    fn rollback_operation(&mut self, op: PendingOperation) {
        match op {
            PendingOperation::TabCreated { tab_id, window_id } => {
                self.windows.detach_tab(&tab_id);
                self.tabs.remove_tab(&tab_id);
                // If this was the only tab in the window, close the window
                if self.tabs.tabs_in_window(&window_id).is_empty() {
                    self.windows.close_window(&window_id);
                }
            }
            PendingOperation::TabMoved {
                tab_id,
                from_window_id,
                to_window_id,
            } => {
                // Move tab back to original window
                self.tabs.move_tab_to_window(&tab_id, &from_window_id);
                self.windows.move_tab_to_window(&tab_id, &from_window_id);
                // If the new window is now empty, close it
                if self.tabs.tabs_in_window(&to_window_id).is_empty() {
                    self.windows.close_window(&to_window_id);
                }
            }
            PendingOperation::TabActivated {
                tab_id: _,
                previous_active_tab_id,
            } => {
                if let Some(prev_tab_id) = previous_active_tab_id {
                    self.tabs.activate_tab(&prev_tab_id);
                }
            }
            PendingOperation::WindowCreated { window_id } => {
                self.windows.close_window(&window_id);
            }
        }
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
        result.map_err(|e| CoreError::Message(e.to_string()))?;
        Ok(Response::Bool(true))
    }

    fn snapshot(&self) -> frost_protocol::AppState {
        let settings = self.build_settings_snapshot();
        frost_protocol::AppState {
            protocol_version: frost_protocol::PROTOCOL_VERSION,
            active_window_id: self.windows.active_window_id().map(ToOwned::to_owned),
            windows: self.windows.list(),
            tabs: self.tabs.list(),
            history: self.repository.list_history().unwrap_or_default(),
            bookmarks: self.repository.list_bookmarks().unwrap_or_default(),
            downloads: self.repository.list_downloads().unwrap_or_default(),
            permissions: self.repository.list_permissions().unwrap_or_default(),
            settings,
        }
    }

    fn build_settings_snapshot(&self) -> serde_json::Value {
        let keys = [
            "homepage",
            "downloadDirectory",
            "searchEngine",
            "startupBehavior",
            "customSearchUrl",
            "theme",
            "appearance",
            "sidebarVisible",
            "sidebarWidth",
            "newTabPage",
            "homeUrl",
            "language",
            "defaultZoomLevel",
            "askBeforeDownload",
        ];
        let mut map = serde_json::Map::new();
        for key in keys {
            if let Ok(Some(value)) = self.repository.get_setting(key) {
                map.insert(key.to_owned(), serde_json::Value::String(value));
            }
        }
        serde_json::Value::Object(map)
    }

    fn emit(&mut self, event: Event) {
        let envelope = EventEnvelope::new(event);
        self.events.push(envelope.clone());
        if self.events.len() > 100 {
            self.events.remove(0);
        }
        if let Some(sender) = &self.event_tx {
            let _ = sender.send(envelope);
        }
    }

    fn trim_closed_tabs(&mut self) {
        if self.closed_tabs.len() > 50 {
            let excess = self.closed_tabs.len() - 50;
            self.closed_tabs.drain(..excess);
        }
    }

    fn record_pending(&mut self, operation: PendingOperation) {
        let id = self
            .adapter
            .last_command_id()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("local-pending-{}", self.pending_operations.len()));
        self.pending_operations.insert(id, operation);
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

struct ClosedWindow {
    window: frost_protocol::WindowState,
    tabs: Vec<frost_protocol::TabState>,
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

        assert_eq!(response.response, Response::Bool(true));
        let command = host_rx.try_recv().unwrap();
        assert!(matches!(
            command.command,
            HostCommand::PageCreate { url, .. } if url == "https://example.com"
        ));
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
    }

    #[test]
    fn adopts_host_created_startup_tab_without_phantom_window() {
        let mut core = BrowserCore::new();

        core.process_host_event(HostEventEnvelope::new(HostEvent::PageCreated {
            tab_id: "host-tab-1".into(),
            window_id: "host-window-1".into(),
            url: "fubuki://newtab/".into(),
            active: true,
            is_private: false,
        }))
        .unwrap();

        let snapshot = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(state) = snapshot.response else {
            panic!("expected snapshot");
        };
        assert_eq!(state.active_window_id.as_deref(), Some("host-window-1"));
        assert_eq!(state.windows.len(), 1);
        assert_eq!(state.windows[0].tab_ids, vec!["host-tab-1"]);
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.tabs[0].window_id, "host-window-1");
        assert!(state.tabs[0].is_active);
    }

    #[test]
    fn host_command_result_ok_is_noop() {
        let mut core = BrowserCore::new();
        let result = HostCommandResultEnvelope {
            version: frost_protocol::PROTOCOL_VERSION,
            command_id: "cmd-1".into(),
            ok: true,
            error: None,
        };
        assert!(core.process_host_command_result(result).is_ok());
    }

    #[test]
    fn host_command_result_error_is_core_error() {
        let mut core = BrowserCore::new();
        let result = HostCommandResultEnvelope {
            version: frost_protocol::PROTOCOL_VERSION,
            command_id: "cmd-2".into(),
            ok: false,
            error: Some("host blew up".into()),
        };
        let err = core.process_host_command_result(result).unwrap_err();
        assert!(err.to_string().contains("cmd-2"));
        assert!(err.to_string().contains("host blew up"));
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

        let startup = core.process(ProtocolRequest::new(Request::SettingsSet {
            key: "startupBehavior".into(),
            value: "restore".into(),
        }));
        assert_eq!(startup.response, Response::Bool(true));
    }

    #[test]
    fn close_other_tabs_emits_page_close_for_every_removed_host_page() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );
        for index in 0..3 {
            core.process(ProtocolRequest::new(Request::TabsCreate {
                url: Some(format!("https://{index}.example")),
                active: true,
                window_id: None,
            }));
        }
        while host_rx.try_recv().is_ok() {}

        let snapshot = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(state) = snapshot.response else {
            panic!("expected snapshot");
        };
        let keep = state.tabs[1].id.clone();
        core.process(ProtocolRequest::new(Request::TabsCloseOther {
            tab_id: keep,
        }));

        let close_count = host_rx
            .try_iter()
            .filter(|command| matches!(command.command, HostCommand::PageClose { .. }))
            .count();
        assert_eq!(close_count, 2);
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
        let resp = core.process(ProtocolRequest::new(Request::WindowsCreate));
        assert_eq!(resp.response, Response::Bool(true));
        let snapshot = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(state) = snapshot.response else {
            panic!("expected snapshot");
        };
        let window_id = state.windows.last().unwrap().id.clone();
        let mut tab_ids = Vec::new();
        for i in 0..count {
            let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
                url: Some(format!("https://example{}.com", i)),
                active: true,
                window_id: Some(window_id.clone()),
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
    fn close_tabs_to_right_scoped_to_window() {
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
        assert_eq!(resp.response, Response::Bool(true));

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        // Window 1 tab should still exist.
        assert!(s.tabs.iter().any(|t| t.id == w1_tab));
        // Window 2 should have only the target tab.
        let w2_id = &s.windows.last().unwrap().id;
        let w2_remaining: Vec<_> = s.tabs.iter().filter(|t| t.window_id == *w2_id).collect();
        assert_eq!(w2_remaining.len(), 1);
        assert_eq!(w2_remaining[0].id, target);
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
    fn move_tab_to_window_deactivates_others() {
        let mut core = BrowserCore::new();
        let (_, w2_tabs) = create_window_with_tabs(&mut core, 2);

        // Move a tab from w2 into a new window via TabsMoveToNewWindow.
        let resp = core.process(ProtocolRequest::new(Request::TabsMoveToNewWindow {
            tab_id: w2_tabs[0].clone(),
        }));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        // The moved tab should be active in its new window.
        let moved = s.tabs.iter().find(|t| t.id == w2_tabs[0]).unwrap();
        assert!(moved.is_active, "moved tab should be active in new window");
        // The remaining tab in w2 should still be active in w2.
        let w2_remaining = s.tabs.iter().find(|t| t.id == w2_tabs[1]).unwrap();
        assert!(
            w2_remaining.is_active,
            "remaining tab in w2 should be active"
        );
        // Exactly 2 active tabs total (one per window).
        let active_count = s.tabs.iter().filter(|t| t.is_active).count();
        assert_eq!(active_count, 2, "one active tab per window");
    }

    #[test]
    fn close_and_reopen_window_preserves_tabs() {
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

        // Reopen the window.
        let resp = core.process(ProtocolRequest::new(Request::WindowsReopenClosed));
        assert_eq!(resp.response, Response::Bool(true));

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let reopened_tabs: Vec<_> = s.tabs.iter().filter(|t| t.window_id == w2_id).collect();
        assert_eq!(reopened_tabs.len(), 2);
        assert!(s.windows.iter().any(|w| w.id == w2_id));
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

    // ── Integration tests for new features ───────────────────────────────────

    #[test]
    fn tabs_create_with_explicit_window_id() {
        let mut core = BrowserCore::new();
        let (w2_id, _) = create_window_with_tabs(&mut core, 1);

        // Create a tab explicitly in window 2
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
            window_id: Some(w2_id.clone()),
        }));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };

        // The new tab should be in window 2
        let new_tab = s.tabs.last().unwrap();
        assert_eq!(new_tab.window_id, w2_id);
        assert_eq!(new_tab.url, "https://example.com");
    }

    #[test]
    fn tabs_create_active_false_does_not_activate_tab() {
        let mut core = BrowserCore::new();
        // Create initial active tab
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://initial.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(resp.ok);

        // Create a tab with active=false
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://inactive.com".into()),
            active: false,
            window_id: None,
        }));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };

        // Only one tab should be active
        let active_tabs: Vec<_> = s.tabs.iter().filter(|t| t.is_active).collect();
        assert_eq!(active_tabs.len(), 1);
        // The active tab should be the initial one, not the new one
        assert_eq!(active_tabs[0].url, "https://initial.com");
    }

    #[test]
    fn move_last_tab_creates_empty_tab_in_source_window() {
        let mut core = BrowserCore::new();
        // Create window 2 with one tab
        let (w2_id, w2_tabs) = create_window_with_tabs(&mut core, 1);

        // Move the only tab from window 2 to a new window
        let resp = core.process(ProtocolRequest::new(Request::TabsMoveToNewWindow {
            tab_id: w2_tabs[0].clone(),
        }));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };

        // Window 2 should still exist with an empty tab
        let w2_tabs_now: Vec<_> = s.tabs.iter().filter(|t| t.window_id == w2_id).collect();
        assert_eq!(w2_tabs_now.len(), 1, "window 2 should have an empty tab");
        assert_eq!(
            w2_tabs_now[0].url, "fubuki://newtab/",
            "empty tab should have newtab URL"
        );
    }

    #[test]
    fn multiple_tabs_restoration() {
        let mut core = BrowserCore::new();
        // Create multiple tabs in window 1
        for i in 0..3 {
            let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
                url: Some(format!("https://tab{}.example.com", i)),
                active: true,
                window_id: None,
            }));
            assert!(resp.ok);
        }

        // Create window 2 with 2 tabs
        let (w2_id, _) = create_window_with_tabs(&mut core, 2);

        // Verify initial state
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let initial_tab_count = s.tabs.len();
        assert_eq!(initial_tab_count, 5); // 3 + 2

        // Close window 2
        let resp = core.process(ProtocolRequest::new(Request::WindowsClose {
            window_id: Some(w2_id.clone()),
        }));
        assert!(resp.ok);

        // Verify window 2 is gone
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        assert_eq!(s.tabs.len(), 3);

        // Restore window 2
        let resp = core.process(ProtocolRequest::new(Request::WindowsReopenClosed));
        assert!(resp.ok);

        // Verify all tabs are restored
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        assert_eq!(s.tabs.len(), initial_tab_count);
        let w2_tabs_restored: Vec<_> = s.tabs.iter().filter(|t| t.window_id == w2_id).collect();
        assert_eq!(w2_tabs_restored.len(), 2);
    }

    #[test]
    fn host_failure_rolls_back_tab_creation() {
        // Create a core with an adapter that always fails
        struct FailingAdapter;
        impl frost_engine_api::EngineAdapter for FailingAdapter {
            fn create_page(
                &mut self,
                _: &str,
                _: &str,
                _: &str,
                _: bool,
            ) -> frost_engine_api::EngineResult<()> {
                Err(frost_engine_api::EngineError::Message(
                    "host failure".into(),
                ))
            }
            fn close_page(&mut self, _: &str) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn activate_page(&mut self, _: &str) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn pin_page(&mut self, _: &str, _: bool) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn move_page(&mut self, _: &str, _: usize) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn move_page_to_window(
                &mut self,
                _: &str,
                _: &str,
            ) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn navigate(&mut self, _: &str, _: &str) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn reload(&mut self, _: &str) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn stop(&mut self, _: &str) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn go_back(&mut self, _: &str) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn go_forward(&mut self, _: &str) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn create_window(&mut self, _: &str, _: bool) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
            fn close_window(&mut self, _: &str) -> frost_engine_api::EngineResult<()> {
                Ok(())
            }
        }

        let mut core =
            BrowserCore::with_adapter_and_settings(FailingAdapter, InMemoryStore::default());

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let initial_tab_count = s.tabs.len();

        // Try to create a tab - should fail and rollback
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(!resp.ok);

        // Verify state is unchanged
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        assert_eq!(s.tabs.len(), initial_tab_count);
    }

    #[test]
    fn host_command_result_failure_triggers_rollback() {
        let mut core = BrowserCore::new();

        // Simulate a host command failure
        let result = HostCommandResultEnvelope {
            version: frost_protocol::PROTOCOL_VERSION,
            command_id: "cmd-fail".into(),
            ok: false,
            error: Some("host command failed".into()),
        };

        let err = core.process_host_command_result(result).unwrap_err();
        assert!(err.to_string().contains("host command failed"));
    }

    #[test]
    fn page_created_event_respects_active_flag() {
        let mut core = BrowserCore::new();

        // Create initial active tab
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://active.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(resp.ok);

        // Simulate host creating a page with active=false
        core.process_host_event(HostEventEnvelope::new(HostEvent::PageCreated {
            tab_id: "host-tab-1".into(),
            window_id: "host-window-1".into(),
            url: "https://inactive.com".into(),
            active: false,
            is_private: false,
        }))
        .unwrap();

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };

        // The original tab should still be active
        let active_tabs: Vec<_> = s.tabs.iter().filter(|t| t.is_active).collect();
        assert_eq!(active_tabs.len(), 1);
        assert_eq!(active_tabs[0].url, "https://active.com");

        // The new tab should not be active
        let host_tab = s.tabs.iter().find(|t| t.id == "host-tab-1").unwrap();
        assert!(!host_tab.is_active);
    }

    #[test]
    fn multiple_windows_operation() {
        let mut core = BrowserCore::new();

        // Create window 1 with 2 tabs
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://w1-tab1.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(resp.ok);
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://w1-tab2.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(resp.ok);

        // Create window 2 with 1 tab
        let (w2_id, _) = create_window_with_tabs(&mut core, 1);

        // Verify initial state
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        assert_eq!(s.windows.len(), 2);
        assert_eq!(s.tabs.len(), 3);

        // Close window 2
        let resp = core.process(ProtocolRequest::new(Request::WindowsClose {
            window_id: Some(w2_id.clone()),
        }));
        assert!(resp.ok);

        // Verify window 2 is gone
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        assert_eq!(s.windows.len(), 1);
        assert_eq!(s.tabs.len(), 2);
    }

    #[test]
    fn private_window_preserves_private_flag() {
        let mut core = BrowserCore::new();

        // Create a private window
        let resp = core.process(ProtocolRequest::new(Request::WindowsCreatePrivate));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };

        // The private window should be marked as private
        let private_windows: Vec<_> = s.windows.iter().filter(|w| w.is_private).collect();
        assert_eq!(private_windows.len(), 1);
    }

    #[test]
    fn popup_tab_registration() {
        let mut core = BrowserCore::new();

        // Simulate a popup tab being created
        core.process_host_event(HostEventEnvelope::new(HostEvent::PageCreated {
            tab_id: "popup-tab-1".into(),
            window_id: "window-1".into(),
            url: "https://popup.example.com".into(),
            active: true,
            is_private: false,
        }))
        .unwrap();

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };

        // The popup tab should be registered
        let popup_tab = s.tabs.iter().find(|t| t.id == "popup-tab-1");
        assert!(popup_tab.is_some());
        assert_eq!(popup_tab.unwrap().url, "https://popup.example.com");
    }

    #[test]
    fn tabs_home_navigates_active_tab() {
        let mut core = BrowserCore::new();

        // Create a tab with a URL
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(resp.ok);

        // Set homeUrl setting
        let resp = core.process(ProtocolRequest::new(Request::SettingsSet {
            key: "homeUrl".into(),
            value: "https://home.example.com".into(),
        }));
        assert!(resp.ok);

        // Execute tabs.home
        let resp = core.process(ProtocolRequest::new(Request::TabsHome {
            tab_id: None,
            window_id: None,
        }));
        assert!(resp.ok);

        // Verify the tab URL changed to home
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let active_tab = s.tabs.iter().find(|t| t.is_active).unwrap();
        assert_eq!(active_tab.url, "https://home.example.com");
    }

    // ── Comprehensive PR #56 tests ──────────────────────────────────────

    /// page.created JSON round-trip: Rust can deserialize what the host sends.
    #[test]
    fn page_created_json_roundtrip() {
        let json = r#"{
            "version": 0,
            "event": "page.created",
            "payload": {
                "tabId": "host-tab-1",
                "windowId": "host-window-1",
                "url": "https://example.com",
                "active": true,
                "isPrivate": false
            }
        }"#;
        let envelope: HostEventEnvelope = serde_json::from_str(json).unwrap();
        match envelope.event {
            HostEvent::PageCreated {
                tab_id,
                window_id,
                url,
                active,
                is_private,
            } => {
                assert_eq!(tab_id, "host-tab-1");
                assert_eq!(window_id, "host-window-1");
                assert_eq!(url, "https://example.com");
                assert!(active);
                assert!(!is_private);
            }
            _ => panic!("expected PageCreated"),
        }
    }

    /// window.closed JSON round-trip.
    #[test]
    fn window_closed_json_roundtrip() {
        let json = r#"{
            "version": 0,
            "event": "window.closed",
            "payload": {
                "windowId": "window-abc"
            }
        }"#;
        let envelope: HostEventEnvelope = serde_json::from_str(json).unwrap();
        match envelope.event {
            HostEvent::WindowClosed { window_id } => {
                assert_eq!(window_id, "window-abc");
            }
            _ => panic!("expected WindowClosed"),
        }
    }

    /// active:false tab creation does NOT change WindowState.activeTabId.
    #[test]
    fn active_false_does_not_change_window_active_tab() {
        let mut core = BrowserCore::new();
        // Create initial active tab
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://active.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(resp.ok);

        // Get the initial active tab id
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let initial_active_id = s.windows[0].active_tab_id.clone().unwrap();

        // Create background tab
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://background.com".into()),
            active: false,
            window_id: None,
        }));
        assert!(resp.ok);

        // WindowState.active_tab_id should NOT change
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        assert_eq!(
            s.windows[0].active_tab_id.as_deref(),
            Some(initial_active_id.as_str())
        );
    }

    /// Background tab creation does not emit tab.activated.
    #[test]
    fn background_tab_no_activation_event() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::new();
        core.set_event_sender(tx);

        // Create initial active tab
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://active.com".into()),
            active: true,
            window_id: None,
        }));
        // Drain events
        while rx.try_recv().is_ok() {}

        // Create background tab
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://background.com".into()),
            active: false,
            window_id: None,
        }));

        // Should only emit tab.created, NOT tab.activated
        let events: Vec<_> = rx.try_iter().collect();
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0].event, Event::TabCreated(_)));
    }

    /// Closing the active tab emits tab.activated for the newly selected tab.
    #[test]
    fn close_active_tab_emits_activation() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::new();
        core.set_event_sender(tx);

        // Create two tabs
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://tab1.com".into()),
            active: true,
            window_id: None,
        }));
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://tab2.com".into()),
            active: true,
            window_id: None,
        }));
        while rx.try_recv().is_ok() {}

        // Get the active tab id
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let active_id = s.tabs.iter().find(|t| t.is_active).unwrap().id.clone();

        // Close the active tab
        core.process(ProtocolRequest::new(Request::TabsClose {
            tab_id: active_id,
        }));

        // Should emit tab.closed AND tab.activated
        let events: Vec<_> = rx.try_iter().collect();
        let has_closed = events
            .iter()
            .any(|e| matches!(&e.event, Event::TabClosed(_)));
        let has_activated = events
            .iter()
            .any(|e| matches!(&e.event, Event::TabActivated(_)));
        assert!(has_closed, "should emit tab.closed");
        assert!(
            has_activated,
            "should emit tab.activated for newly selected tab"
        );
    }

    /// Moving the last tab to a new window creates an active empty tab in the source.
    #[test]
    fn move_last_tab_creates_active_empty_tab() {
        let mut core = BrowserCore::new();
        let (w2_id, w2_tabs) = create_window_with_tabs(&mut core, 1);

        // Move the only tab from w2 to a new window
        let resp = core.process(ProtocolRequest::new(Request::TabsMoveToNewWindow {
            tab_id: w2_tabs[0].clone(),
        }));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };

        // The empty tab in w2 should be active
        let w2_tabs_now: Vec<_> = s.tabs.iter().filter(|t| t.window_id == w2_id).collect();
        assert_eq!(w2_tabs_now.len(), 1);
        assert!(w2_tabs_now[0].is_active, "empty tab should be active");
        assert_eq!(w2_tabs_now[0].url, "fubuki://newtab/");
    }

    /// TabsMove updates WindowState.tabIds order.
    #[test]
    fn tabs_move_updates_window_tab_ids() {
        let mut core = BrowserCore::new();
        let (w_id, tabs) = create_window_with_tabs(&mut core, 3);
        let first_tab = tabs[0].clone();

        // Move first tab to index 2
        let resp = core.process(ProtocolRequest::new(Request::TabsMove {
            tab_id: first_tab.clone(),
            to_index: 2,
        }));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let window = s.windows.iter().find(|w| w.id == w_id).unwrap();
        // The moved tab should be last in the window's tab_ids
        assert_eq!(window.tab_ids.last().unwrap(), &first_tab);
    }

    /// Invalid window_id produces an error.
    #[test]
    fn invalid_window_id_errors() {
        let mut core = BrowserCore::new();
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
            window_id: Some("nonexistent-window".into()),
        }));
        assert!(!resp.ok);
        match resp.response {
            Response::Error(msg) => assert!(msg.contains("does not exist")),
            _ => panic!("expected error response"),
        }
    }

    /// TabsActivateNext cycles through tabs in the window.
    #[test]
    fn activate_next_cycles_tabs() {
        let mut core = BrowserCore::new();
        let (_, w_tabs) = create_window_with_tabs(&mut core, 3);

        // Activate first tab
        core.process(ProtocolRequest::new(Request::TabsActivate {
            tab_id: w_tabs[0].clone(),
        }));

        // Activate next
        let resp = core.process(ProtocolRequest::new(Request::TabsActivateNext));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let active = s.tabs.iter().find(|t| t.is_active).unwrap();
        assert_eq!(active.id, w_tabs[1]);
    }

    /// TabsActivatePrevious cycles backwards.
    #[test]
    fn activate_previous_cycles_tabs() {
        let mut core = BrowserCore::new();
        let (_, w_tabs) = create_window_with_tabs(&mut core, 3);

        // Activate first tab
        core.process(ProtocolRequest::new(Request::TabsActivate {
            tab_id: w_tabs[0].clone(),
        }));

        // Activate previous (should wrap to last)
        let resp = core.process(ProtocolRequest::new(Request::TabsActivatePrevious));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let active = s.tabs.iter().find(|t| t.is_active).unwrap();
        assert_eq!(active.id, w_tabs[2]);
    }

    /// TabsUnpin works through FrostEngine.
    #[test]
    fn unpin_tab_through_engine() {
        let mut core = BrowserCore::new();
        let (_, w_tabs) = create_window_with_tabs(&mut core, 1);

        // Pin the tab
        core.process(ProtocolRequest::new(Request::TabsPin {
            tab_id: w_tabs[0].clone(),
            pinned: true,
        }));

        // Unpin via TabsUnpin
        let resp = core.process(ProtocolRequest::new(Request::TabsUnpin {
            tab_id: w_tabs[0].clone(),
        }));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        assert!(!s.tabs[0].is_pinned);
    }

    /// MoveToNewWindow preserves private flag.
    #[test]
    fn move_to_new_window_preserves_private() {
        let mut core = BrowserCore::new();
        // Create a private window
        let resp = core.process(ProtocolRequest::new(Request::WindowsCreatePrivate));
        assert!(resp.ok);
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let private_w = s.windows.iter().find(|w| w.is_private).unwrap();
        let private_w_id = private_w.id.clone();

        // Create a tab in the private window
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://private.example.com".into()),
            active: true,
            window_id: Some(private_w_id),
        }));
        assert!(resp.ok);
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let private_w = s.windows.iter().find(|w| w.is_private).unwrap();
        let tab_id = private_w.tab_ids[0].clone();

        // Move the tab to a new window
        let resp = core.process(ProtocolRequest::new(Request::TabsMoveToNewWindow {
            tab_id: tab_id.clone(),
        }));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        // The new window should also be private
        let moved_tab = s.tabs.iter().find(|t| t.id == tab_id).unwrap();
        let new_window = s
            .windows
            .iter()
            .find(|w| w.id == moved_tab.window_id)
            .unwrap();
        assert!(
            new_window.is_private,
            "new window should preserve private flag"
        );
    }

    /// Duplicate tab creates a background tab (not active).
    #[test]
    fn duplicate_creates_background_tab() {
        let mut core = BrowserCore::new();
        let (_, w_tabs) = create_window_with_tabs(&mut core, 1);

        // Duplicate the tab
        let resp = core.process(ProtocolRequest::new(Request::TabsDuplicate {
            tab_id: w_tabs[0].clone(),
        }));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        // Only one tab should be active (the original)
        let active_tabs: Vec<_> = s.tabs.iter().filter(|t| t.is_active).collect();
        assert_eq!(active_tabs.len(), 1);
        assert_eq!(active_tabs[0].id, w_tabs[0]);
    }

    /// closeOther and closeToRight maintain active tab consistency.
    #[test]
    fn close_other_maintains_active() {
        let mut core = BrowserCore::new();
        let (w_id, w_tabs) = create_window_with_tabs(&mut core, 3);

        // Keep the middle tab
        core.process(ProtocolRequest::new(Request::TabsCloseOther {
            tab_id: w_tabs[1].clone(),
        }));

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        // The kept tab should be active
        let active = s.tabs.iter().find(|t| t.is_active).unwrap();
        assert_eq!(active.id, w_tabs[1]);
        // WindowState.activeTabId should match
        let window = s.windows.iter().find(|w| w.id == w_id).unwrap();
        assert_eq!(window.active_tab_id.as_deref(), Some(w_tabs[1].as_str()));
    }

    /// closeToRight maintains active tab consistency.
    #[test]
    fn close_to_right_maintains_active() {
        let mut core = BrowserCore::new();
        let (w_id, w_tabs) = create_window_with_tabs(&mut core, 3);

        // Close tabs to right of the first tab
        core.process(ProtocolRequest::new(Request::TabsCloseToRight {
            tab_id: w_tabs[0].clone(),
        }));

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let active = s.tabs.iter().find(|t| t.is_active).unwrap();
        assert_eq!(active.id, w_tabs[0]);
        let window = s.windows.iter().find(|w| w.id == w_id).unwrap();
        assert_eq!(window.active_tab_id.as_deref(), Some(w_tabs[0].as_str()));
    }

    /// Private window popup preserves is_private.
    #[test]
    fn private_popup_preserves_is_private() {
        let mut core = BrowserCore::new();
        // Create a private window
        let resp = core.process(ProtocolRequest::new(Request::WindowsCreatePrivate));
        assert!(resp.ok);
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let private_w = s.windows.iter().find(|w| w.is_private).unwrap();
        let private_w_id = private_w.id.clone();

        // Simulate a popup in the private window
        core.process_host_event(HostEventEnvelope::new(HostEvent::PageCreated {
            tab_id: "popup-in-private".into(),
            window_id: private_w_id.clone(),
            url: "https://popup.example.com".into(),
            active: true,
            is_private: true,
        }))
        .unwrap();

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        // The window should still be private
        let w = s.windows.iter().find(|w| w.id == private_w_id).unwrap();
        assert!(w.is_private);
    }

    /// Multiple windows with independent active tabs.
    #[test]
    fn multiple_windows_independent_active_tabs() {
        let mut core = BrowserCore::new();
        // Create window 1 with a tab
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://w1.com".into()),
            active: true,
            window_id: None,
        }));
        assert!(resp.ok);

        // Create window 2 with a tab
        let (w2_id, _w2_tabs) = create_window_with_tabs(&mut core, 1);

        // Both windows should have an active tab
        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        let active_count = s.tabs.iter().filter(|t| t.is_active).count();
        assert_eq!(active_count, 2, "each window should have one active tab");

        // Activate a different tab in window 2
        // First add another tab to w2
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://w2-tab2.com".into()),
            active: true,
            window_id: Some(w2_id.clone()),
        }));
        assert!(resp.ok);

        let snap = core.process(ProtocolRequest::new(Request::AppSnapshot));
        let Response::AppSnapshot(s) = snap.response else {
            panic!()
        };
        // Window 2 should still have exactly one active tab
        let w2_tabs: Vec<_> = s.tabs.iter().filter(|t| t.window_id == w2_id).collect();
        let w2_active: Vec<_> = w2_tabs.iter().filter(|t| t.is_active).collect();
        assert_eq!(w2_active.len(), 1);
    }
}
