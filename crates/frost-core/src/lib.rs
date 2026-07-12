mod bookmark_service;
mod download_service;
mod external_router;
mod history_service;
mod settings_service;
mod tab_service;
mod window_service;

pub use external_router::{ExternalPolicy, ExternalResponse};

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, Sender};
use frost_engine_api::{EngineAdapter, EngineError, EngineResult, NoopEngineAdapter};
use frost_protocol::{
    BrowserCommand, Event, EventEnvelope, HostCommand, HostCommandEnvelope,
    HostCommandResultEnvelope, HostEvent, HostEventEnvelope, ProtocolRequest, ProtocolResponse,
    Request, Response, SettingChanged, TabActivated, TabClosed, TabPatch,
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
    result_rx: Option<Receiver<HostCommandResultEnvelope>>,
    notify: Option<Arc<dyn Fn() + Send + Sync>>,
    pending_results: Arc<Mutex<HashMap<String, HostCommandResultEnvelope>>>,
}

impl HostCommandAdapter {
    pub fn new(tx: Sender<HostCommandEnvelope>) -> Self {
        Self {
            tx,
            result_rx: None,
            notify: None,
            pending_results: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn with_results(
        tx: Sender<HostCommandEnvelope>,
        result_rx: Receiver<HostCommandResultEnvelope>,
        notify: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            tx,
            result_rx: Some(result_rx),
            notify: Some(notify),
            pending_results: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn send(&self, command: HostCommand) -> EngineResult<()> {
        let id = format!("host-command-{}", uuid::Uuid::new_v4());
        self.tx
            .send(HostCommandEnvelope::new(id.clone(), command))
            .map_err(|e| EngineError::Message(e.to_string()))?;
        if let Some(notify) = &self.notify {
            notify();
        }
        let Some(result_rx) = &self.result_rx else {
            return Ok(());
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        let result = loop {
            if let Some(result) = self
                .pending_results
                .lock()
                .map_err(|_| EngineError::Message("host result map poisoned".into()))?
                .remove(&id)
            {
                break result;
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(EngineError::Message(format!("host command {id} timed out")));
            }
            match result_rx.recv_timeout(remaining) {
                Ok(result) if result.command_id == id => break result,
                Ok(result) => {
                    self.pending_results
                        .lock()
                        .map_err(|_| EngineError::Message("host result map poisoned".into()))?
                        .entry(result.command_id.clone())
                        .or_insert(result);
                }
                Err(error) => {
                    return Err(EngineError::Message(format!(
                        "host command {id} timed out: {error}"
                    )));
                }
            }
        };
        if result.ok {
            Ok(())
        } else {
            Err(EngineError::Message(
                result
                    .error
                    .unwrap_or_else(|| format!("host command {id} failed")),
            ))
        }
    }
}

impl EngineAdapter for HostCommandAdapter {
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

    fn close_page(
        &mut self,
        tab_id: &str,
        window_id: Option<&str>,
        successor_tab_id: Option<&str>,
    ) -> EngineResult<()> {
        self.send(HostCommand::PageClose {
            tab_id: tab_id.to_owned(),
            window_id: window_id.map(ToOwned::to_owned),
            successor_tab_id: successor_tab_id.map(ToOwned::to_owned),
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

    fn set_overlay(
        &mut self,
        window_id: &str,
        active: bool,
        width: Option<f64>,
        height: Option<f64>,
    ) -> EngineResult<()> {
        self.send(HostCommand::UiOverlaySet {
            window_id: window_id.to_owned(),
            active,
            width,
            height,
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

    fn apply_setting(&mut self, key: &str, value: &str) -> EngineResult<()> {
        self.send(HostCommand::SettingsApply {
            key: key.to_owned(),
            value: value.to_owned(),
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
    session_dirty_since: Option<Instant>,
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
            session_dirty_since: None,
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
        let session_mutation = matches!(
            &request,
            Request::TabsCreate { .. }
                | Request::TabsActivate { .. }
                | Request::TabsClose { .. }
                | Request::TabsPin { .. }
                | Request::TabsDuplicate { .. }
                | Request::TabsReopenClosed
                | Request::TabsCloseOther { .. }
                | Request::TabsCloseToRight { .. }
                | Request::TabsMove { .. }
                | Request::TabsMoveToNewWindow { .. }
                | Request::TabsNavigate { .. }
                | Request::WindowsCreate
                | Request::WindowsCreatePrivate
                | Request::WindowsClose { .. }
                | Request::WindowsReopenClosed
        );
        let force_session_flush = matches!(&request, Request::WindowsClose { .. });
        let rollback_tabs = self.tabs.list();
        let rollback_windows = self.windows.list();
        let rollback_active_window = self.windows.active_window_id().map(ToOwned::to_owned);
        let rollback_closed_tabs = self.closed_tabs.clone();
        let rollback_closed_windows = self.closed_windows.clone();
        let rollback_event_count = self.events.len();
        let result = (|| -> CoreResult<Response> {
            match request {
                Request::AppStartup => self.startup(),
                Request::AppSnapshot => Ok(Response::AppSnapshot(self.snapshot())),
                Request::TabsList => Ok(Response::TabsList(self.tabs.list())),
                Request::TabsCreate { url, active } => {
                    let window_id = self
                        .windows
                        .active_window_id()
                        .ok_or_else(|| CoreError::Message("No active window".into()))?
                        .to_owned();
                    let tab = self.tabs.create_tab(
                        window_id.clone(),
                        url.unwrap_or_else(|| "fubuki://newtab/".into()),
                        active,
                    );
                    self.windows.attach_tab(&window_id, &tab.id, tab.is_active);
                    if let Err(e) =
                        self.adapter
                            .create_page(&tab.id, &window_id, &tab.url, tab.is_active)
                    {
                        self.windows.detach_tab(&tab.id);
                        self.tabs.remove_tab(&tab.id);
                        return Err(CoreError::Message(e.to_string()));
                    }
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
                    let tabs_before = self.tabs.list();
                    let windows_before = self.windows.list();
                    let active_window_before = self.windows.active_window_id().map(str::to_owned);
                    let closed_tabs_before = self.closed_tabs.clone();
                    let closing_tab = self.tabs.get_tab(&tab_id);
                    if let Some(tab) = closing_tab.clone() {
                        self.closed_tabs.push(tab);
                        if self.closed_tabs.len() > 50 {
                            self.closed_tabs.remove(0);
                        }
                    }
                    let close_outcome = self.tabs.close_tab(&tab_id);
                    if let Some(successor_tab_id) = close_outcome.as_ref() {
                        self.windows.detach_tab(&tab_id);
                        if let Some(successor_tab_id) = successor_tab_id {
                            self.windows.set_active_tab(successor_tab_id);
                        }
                        self.adapter
                            .close_page(
                                &tab_id,
                                closing_tab.as_ref().map(|tab| tab.window_id.as_str()),
                                successor_tab_id.as_deref(),
                            )
                            .map_err(|e| {
                                // Roll back state on close_page failure
                                self.tabs.replace_all(tabs_before.clone());
                                self.windows.replace_all(
                                    windows_before.clone(),
                                    active_window_before.clone(),
                                );
                                self.closed_tabs = closed_tabs_before.clone();
                                CoreError::Message(e.to_string())
                            })?;
                        self.emit(Event::TabClosed(TabClosed { tab_id }));
                        if let Some(successor_tab_id) = successor_tab_id {
                            self.emit(Event::TabActivated(TabActivated {
                                tab_id: successor_tab_id.clone(),
                            }));
                        }

                        // Keep the browser usable, but do so in the engine rather
                        // than letting the native tab manager invent a second tab.
                        if let Some(tab) = closing_tab
                            && !self.tabs.has_tabs_in_window(&tab.window_id)
                        {
                            let replacement = self.tabs.create_tab(
                                tab.window_id.clone(),
                                "fubuki://newtab/".into(),
                                true,
                            );
                            self.windows
                                .attach_tab(&tab.window_id, &replacement.id, true);
                            if let Err(replacement_err) = self.adapter.create_page(
                                &replacement.id,
                                &tab.window_id,
                                &replacement.url,
                                replacement.is_active,
                            ) {
                                // Roll back replacement state
                                self.windows.detach_tab(&replacement.id);
                                self.tabs.remove_tab(&replacement.id);
                                // Roll back the entire close operation
                                self.tabs.replace_all(tabs_before);
                                self.windows
                                    .replace_all(windows_before, active_window_before);
                                self.closed_tabs = closed_tabs_before;
                                // Compensation: recreate the original tab in native
                                let compensation_result = self.adapter.create_page(
                                    &tab.id,
                                    &tab.window_id,
                                    &tab.url,
                                    tab.is_active,
                                );
                                return match compensation_result {
                                    Ok(()) => Err(CoreError::Message(format!(
                                        "replacement page creation failed: {replacement_err}"
                                    ))),
                                    Err(compensation_err) => Err(CoreError::Message(format!(
                                        "replacement failed: {replacement_err}, \
                                             compensation also failed: {compensation_err}"
                                    ))),
                                };
                            }
                            self.emit(Event::TabCreated(replacement));
                        }
                    }
                    Ok(Response::Bool(close_outcome.is_some()))
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
                    self.windows
                        .attach_tab(&tab.window_id, &tab.id, tab.is_active);
                    if let Err(e) =
                        self.adapter
                            .create_page(&tab.id, &tab.window_id, &tab.url, tab.is_active)
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
                    let window_id = tab.window_id.clone();
                    let created = tab.clone();
                    self.tabs.upsert_tab(tab);
                    self.windows.attach_tab(&window_id, &created.id, true);
                    if let Err(e) = self.adapter.create_page(
                        &created.id,
                        &window_id,
                        &created.url,
                        created.is_active,
                    ) {
                        self.windows.detach_tab(&created.id);
                        self.tabs.remove_tab(&created.id);
                        return Err(CoreError::Message(e.to_string()));
                    }
                    self.emit(Event::TabCreated(created));
                    Ok(Response::Bool(true))
                }
                Request::TabsCloseOther { tab_id } => {
                    if !self.tabs.contains(&tab_id) {
                        return Ok(Response::Bool(false));
                    }
                    let closed = self.tabs.close_other_tabs(&tab_id);
                    let active_tab_was_closed = closed.iter().any(|tab| tab.is_active);
                    self.windows.set_active_tab(&tab_id);
                    for tab in &closed {
                        self.windows.detach_tab(&tab.id);
                    }
                    self.close_pages_with_compensation(
                        &closed,
                        active_tab_was_closed.then_some(tab_id.as_str()),
                    )?;
                    for tab in &closed {
                        self.emit(Event::TabClosed(TabClosed {
                            tab_id: tab.id.clone(),
                        }));
                    }
                    self.closed_tabs.extend(closed);
                    self.trim_closed_tabs();
                    if active_tab_was_closed {
                        self.emit(Event::TabActivated(TabActivated { tab_id }));
                    }
                    Ok(Response::Bool(true))
                }
                Request::TabsCloseToRight { tab_id } => {
                    if !self.tabs.contains(&tab_id) {
                        return Ok(Response::Bool(false));
                    }
                    let closed = self.tabs.close_tabs_to_right(&tab_id);
                    let active_tab_was_closed = closed.iter().any(|tab| tab.is_active);
                    if active_tab_was_closed {
                        self.tabs.activate_tab(&tab_id);
                        self.windows.set_active_tab(&tab_id);
                    }
                    for tab in &closed {
                        self.windows.detach_tab(&tab.id);
                    }
                    self.close_pages_with_compensation(
                        &closed,
                        active_tab_was_closed.then_some(tab_id.as_str()),
                    )?;
                    for tab in &closed {
                        self.emit(Event::TabClosed(TabClosed {
                            tab_id: tab.id.clone(),
                        }));
                    }
                    self.closed_tabs.extend(closed);
                    self.trim_closed_tabs();
                    if active_tab_was_closed {
                        self.emit(Event::TabActivated(TabActivated { tab_id }));
                    }
                    Ok(Response::Bool(true))
                }
                Request::TabsMove { tab_id, to_index } => {
                    let ok = self.tabs.move_tab(&tab_id, to_index);
                    if ok {
                        self.adapter
                            .move_page(&tab_id, to_index)
                            .map_err(|e| CoreError::Message(e.to_string()))?;
                        self.emit(Event::TabUpdated(TabPatch {
                            tab_id,
                            ..Default::default()
                        }));
                    }
                    Ok(Response::Bool(ok))
                }
                Request::TabsMoveToNewWindow { tab_id } => {
                    if !self.tabs.contains(&tab_id) {
                        return Ok(Response::Bool(false));
                    }
                    let moving_tab = self
                        .tabs
                        .get_tab(&tab_id)
                        .ok_or_else(|| CoreError::Message("Tab not found".into()))?;
                    let previous_window = moving_tab.window_id.clone();
                    let url = moving_tab.url.clone();
                    let window_id = self.windows.create_window(false);
                    self.tabs.move_tab_to_window(&tab_id, &window_id);
                    self.windows.move_tab_to_window(&tab_id, &window_id);
                    let source_successor = self
                        .windows
                        .get_window(&previous_window)
                        .and_then(|window| window.active_tab_id);
                    self.adapter
                        .create_window(&window_id, false)
                        .map_err(|error| CoreError::Message(error.to_string()))?;
                    if let Err(error) = self.adapter.create_page(&tab_id, &window_id, &url, true) {
                        return Err(self.compensate_window(error, &window_id));
                    }
                    if let Err(error) = self.adapter.close_page(
                        &tab_id,
                        Some(&previous_window),
                        source_successor.as_deref(),
                    ) {
                        let mut compensation_errors = Vec::new();
                        if let Err(compensation) =
                            self.adapter.close_page(&tab_id, Some(&window_id), None)
                        {
                            compensation_errors.push(compensation.to_string());
                        }
                        if let Err(compensation) = self.adapter.close_window(&window_id) {
                            compensation_errors.push(compensation.to_string());
                        }
                        return Err(Self::compensation_error(error, compensation_errors));
                    }
                    if let Some(window) = self.windows.get_window(&window_id) {
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
                Request::TabsReload { tab_id } => {
                    self.host_tab_action(&tab_id, HostTabAction::Reload)
                }
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
                Request::TabsGoBack { tab_id } => {
                    self.host_tab_action(&tab_id, HostTabAction::GoBack)
                }
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
                    let window_id = self.windows.create_window(false);
                    // Automatically create a startup tab for the new window.
                    let startup_url = self.startup_url();
                    let tab = self.tabs.create_tab(window_id.clone(), startup_url, true);
                    self.windows.attach_tab(&window_id, &tab.id, true);
                    self.adapter
                        .create_window(&window_id, false)
                        .map_err(|error| CoreError::Message(error.to_string()))?;
                    if let Err(error) =
                        self.adapter
                            .create_page(&tab.id, &window_id, &tab.url, tab.is_active)
                    {
                        return Err(self.compensate_window(error, &window_id));
                    }
                    if let Some(window) = self.windows.get_window(&window_id) {
                        self.emit(Event::WindowCreated(window));
                    }
                    self.emit(Event::TabCreated(tab));
                    Ok(Response::Bool(true))
                }
                Request::WindowsCreatePrivate => {
                    let window_id = self.windows.create_window(true);
                    // Automatically create a startup tab for the new private window.
                    let startup_url = self.startup_url();
                    let tab = self.tabs.create_tab(window_id.clone(), startup_url, true);
                    self.windows.attach_tab(&window_id, &tab.id, true);
                    self.adapter
                        .create_window(&window_id, true)
                        .map_err(|error| CoreError::Message(error.to_string()))?;
                    if let Err(error) =
                        self.adapter
                            .create_page(&tab.id, &window_id, &tab.url, tab.is_active)
                    {
                        return Err(self.compensate_window(error, &window_id));
                    }
                    if let Some(window) = self.windows.get_window(&window_id) {
                        self.emit(Event::WindowCreated(window));
                    }
                    self.emit(Event::TabCreated(tab));
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
                        if !window_state.is_private {
                            self.closed_windows.push(ClosedWindow {
                                window: window_state,
                                tabs: window_tabs.clone(),
                            });
                            if self.closed_windows.len() > 10 {
                                self.closed_windows.remove(0);
                            }
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
                    self.adapter
                        .create_window(&closed.window.id, closed.window.is_private)
                        .map_err(|error| CoreError::Message(error.to_string()))?;
                    for tab in &closed.tabs {
                        if let Err(error) = self.adapter.create_page(
                            &tab.id,
                            &tab.window_id,
                            &tab.url,
                            tab.is_active,
                        ) {
                            return Err(self.compensate_window(error, &closed.window.id));
                        }
                    }
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
                    SettingsService::validate(&key, &value)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.persist_and_apply_setting(&key, &value)?;
                    self.emit(Event::SettingChanged(SettingChanged { key, value }));
                    Ok(Response::Bool(true))
                }
                Request::SettingsReset { key } => {
                    let value = SettingsService::default_value(&key);
                    SettingsService::validate(&key, value)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.persist_and_apply_setting(&key, value)?;
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
                    let ok =
                        DownloadService::remove(&self.repository, url.as_deref(), path.as_deref())
                            .map_err(|e| CoreError::Message(e.to_string()))?;
                    if ok {
                        self.emit(Event::DownloadChanged { url, path });
                    }
                    Ok(Response::Bool(ok))
                }
                Request::DataClear { target } => {
                    let target = target.unwrap_or_else(|| "all".into());
                    self.clear_browsing_data(&target)?;
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
                Request::CommandsExecute { id, args } => {
                    let argument = |name: &str| args.as_ref().and_then(|value| value.get(name));
                    let active_tab_id = self.tabs.active_tab().map(|tab| tab.id);
                    let tab_argument = || {
                        argument("tabId")
                            .and_then(|value| value.as_str())
                            .map(str::to_owned)
                            .or_else(|| active_tab_id.clone())
                    };
                    let delegated = match id.as_str() {
                        "tabs.create" => Some(Request::TabsCreate {
                            url: argument("url")
                                .and_then(|value| value.as_str())
                                .map(str::to_owned),
                            active: argument("active")
                                .and_then(|value| value.as_bool())
                                .unwrap_or(true),
                        }),
                        "tabs.close" => tab_argument().map(|tab_id| Request::TabsClose { tab_id }),
                        "tabs.pin" | "tabs.unpin" => {
                            tab_argument().map(|tab_id| Request::TabsPin {
                                tab_id,
                                pinned: id == "tabs.pin",
                            })
                        }
                        "tabs.closeOther" => {
                            tab_argument().map(|tab_id| Request::TabsCloseOther { tab_id })
                        }
                        "tabs.closeToRight" => {
                            tab_argument().map(|tab_id| Request::TabsCloseToRight { tab_id })
                        }
                        "tabs.reopenClosed" => Some(Request::TabsReopenClosed),
                        "tabs.duplicate" => {
                            tab_argument().map(|tab_id| Request::TabsDuplicate { tab_id })
                        }
                        "tabs.moveToNewWindow" => {
                            tab_argument().map(|tab_id| Request::TabsMoveToNewWindow { tab_id })
                        }
                        "tabs.reload" => {
                            tab_argument().map(|tab_id| Request::TabsReload { tab_id })
                        }
                        "windows.create" => Some(Request::WindowsCreate),
                        "windows.createPrivate" => Some(Request::WindowsCreatePrivate),
                        "windows.close" => Some(Request::WindowsClose {
                            window_id: argument("windowId")
                                .and_then(|value| value.as_str())
                                .map(str::to_owned),
                        }),
                        "windows.reopenClosed" => Some(Request::WindowsReopenClosed),
                        _ => None,
                    };
                    match delegated {
                        Some(request) => self.process_inner(request),
                        None => Ok(Response::Json(serde_json::json!({
                            "handled": false,
                            "id": id
                        }))),
                    }
                }
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
                    active,
                    width,
                    height,
                } => {
                    let window_id = self
                        .windows
                        .active_window_id()
                        .ok_or_else(|| CoreError::Message("No active window".into()))?;
                    self.adapter
                        .set_overlay(window_id, active, width, height)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    Ok(Response::Bool(true))
                }
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
                        let invalid_settings: Vec<String> = settings
                            .iter()
                            .filter_map(|(key, value)| {
                                let value = value.as_str()?;
                                SettingsService::validate(key, value)
                                    .err()
                                    .map(|error| format!("setting '{}': {}", key, error))
                            })
                            .collect();
                        if invalid_settings.is_empty() {
                            for (key, value) in settings {
                                if let Some(value) = value.as_str()
                                    && let Err(e) = self.repository.set_setting(key, value)
                                {
                                    errors.push(format!("setting '{}': {}", key, e));
                                }
                            }
                        } else {
                            errors.extend(invalid_settings);
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
            }
        })();
        if result.is_err() {
            self.tabs.replace_all(rollback_tabs);
            self.windows
                .replace_all(rollback_windows, rollback_active_window);
            self.closed_tabs = rollback_closed_tabs;
            self.closed_windows = rollback_closed_windows;
            self.events.truncate(rollback_event_count);
        } else if session_mutation {
            self.mark_session_dirty();
            if force_session_flush {
                self.flush_session(true)?;
            }
        }
        result
    }

    pub fn process_host_event(&mut self, envelope: HostEventEnvelope) -> CoreResult<()> {
        let changes_session = matches!(
            &envelope.event,
            HostEvent::PageCreated { .. }
                | HostEvent::PageClosed { .. }
                | HostEvent::PageTitleChanged { .. }
                | HostEvent::PageUrlChanged { .. }
                | HostEvent::PageFaviconChanged { .. }
                | HostEvent::WindowFocused { .. }
                | HostEvent::WindowClosed { .. }
        );
        let force = matches!(&envelope.event, HostEvent::WindowClosed { .. });
        let result = match envelope.event {
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
                }
                Ok(())
            }
            HostEvent::PageClosed { tab_id } => {
                let close_outcome = self.tabs.close_tab(&tab_id);
                if let Some(successor_tab_id) = close_outcome {
                    self.windows.detach_tab(&tab_id);
                    if let Some(successor_tab_id) = successor_tab_id.as_ref() {
                        self.windows.set_active_tab(successor_tab_id);
                    }
                    self.emit(Event::TabClosed(TabClosed { tab_id }));
                    if let Some(successor_tab_id) = successor_tab_id {
                        self.emit(Event::TabActivated(TabActivated {
                            tab_id: successor_tab_id,
                        }));
                    }
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
            HostEvent::WindowClosed { window_id } => {
                let window_state = self.windows.get_window(&window_id);
                let window_tabs: Vec<frost_protocol::TabState> = self
                    .tabs
                    .list()
                    .into_iter()
                    .filter(|tab| tab.window_id == window_id)
                    .collect();
                if self.windows.close_window(&window_id) {
                    if let Some(window) = window_state.filter(|window| !window.is_private) {
                        self.closed_windows.push(ClosedWindow {
                            window,
                            tabs: window_tabs.clone(),
                        });
                        if self.closed_windows.len() > 10 {
                            self.closed_windows.remove(0);
                        }
                    }
                    for tab in window_tabs {
                        self.tabs.remove_tab(&tab.id);
                        self.emit(Event::TabClosed(TabClosed { tab_id: tab.id }));
                    }
                    self.emit(Event::WindowClosed { window_id });
                }
                Ok(())
            }
        };
        if result.is_ok() && changes_session {
            self.mark_session_dirty();
            if force {
                self.flush_session(true)?;
            }
        }
        result
    }

    pub fn process_host_command_result(
        &mut self,
        result: HostCommandResultEnvelope,
    ) -> CoreResult<()> {
        if result.ok {
            return Ok(());
        }
        Err(CoreError::Message(format!(
            "Host command {} failed: {}",
            result.command_id,
            result.error.unwrap_or_else(|| "unknown error".into())
        )))
    }

    fn persist_and_apply_setting(&mut self, key: &str, value: &str) -> CoreResult<()> {
        let previous = self
            .repository
            .get_setting(key)
            .map_err(|error| CoreError::Message(error.to_string()))?;
        self.repository
            .set_setting(key, value)
            .map_err(|error| CoreError::Message(error.to_string()))?;
        if let Err(host_error) = self.adapter.apply_setting(key, value) {
            let previous_host_value = previous
                .as_deref()
                .unwrap_or_else(|| SettingsService::default_value(key));
            let mut compensation_errors = Vec::new();
            if let Err(error) = self.adapter.apply_setting(key, previous_host_value) {
                compensation_errors.push(error.to_string());
            }
            let rollback = match previous.as_deref() {
                Some(previous) => self.repository.set_setting(key, previous),
                None => self.repository.remove_setting(key),
            };
            if let Err(error) = rollback {
                compensation_errors.push(error.to_string());
            }
            return Err(Self::compensation_error(host_error, compensation_errors));
        }
        Ok(())
    }

    fn compensate_window(&mut self, error: EngineError, window_id: &str) -> CoreError {
        self.compensate_windows(error, &[window_id.to_owned()])
    }

    fn close_pages_with_compensation(
        &mut self,
        tabs: &[frost_protocol::TabState],
        successor_tab_id: Option<&str>,
    ) -> CoreResult<()> {
        let mut closed = Vec::new();
        for tab in tabs {
            if let Err(error) = self.adapter.close_page(
                &tab.id,
                Some(&tab.window_id),
                tab.is_active.then_some(successor_tab_id).flatten(),
            ) {
                let compensation_errors = closed
                    .iter()
                    .filter_map(|closed: &&frost_protocol::TabState| {
                        self.adapter
                            .create_page(
                                &closed.id,
                                &closed.window_id,
                                &closed.url,
                                closed.is_active,
                            )
                            .err()
                    })
                    .map(|error| error.to_string())
                    .collect();
                return Err(Self::compensation_error(error, compensation_errors));
            }
            closed.push(tab);
        }
        Ok(())
    }

    fn compensate_windows(&mut self, error: EngineError, window_ids: &[String]) -> CoreError {
        let compensation_errors = window_ids
            .iter()
            .rev()
            .filter_map(|window_id| self.adapter.close_window(window_id).err())
            .map(|error| error.to_string())
            .collect();
        Self::compensation_error(error, compensation_errors)
    }

    fn compensation_error(error: EngineError, compensation_errors: Vec<String>) -> CoreError {
        if compensation_errors.is_empty() {
            CoreError::Message(error.to_string())
        } else {
            CoreError::Message(format!(
                "{error}; host compensation also failed: {}",
                compensation_errors.join("; ")
            ))
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

    fn startup(&mut self) -> CoreResult<Response> {
        let behavior = SettingsService::get(&self.repository, "startupBehavior")
            .map_err(|e| CoreError::Message(e.to_string()))?
            .unwrap_or_else(|| "newTab".into());

        if behavior == "restore"
            && let Some(json) = self
                .repository
                .get_session()
                .map_err(|e| CoreError::Message(e.to_string()))?
            && let Ok(session) = serde_json::from_str::<SessionState>(&json)
            && !session.windows.is_empty()
        {
            self.windows
                .replace_all(session.windows.clone(), session.active_window_id);
            self.tabs.replace_all(session.tabs.clone());
            let mut created_window_ids = Vec::new();
            for window in &session.windows {
                if let Err(error) = self.adapter.create_window(&window.id, window.is_private) {
                    return Err(self.compensate_windows(error, &created_window_ids));
                }
                created_window_ids.push(window.id.clone());
            }
            for tab in &session.tabs {
                if let Err(error) =
                    self.adapter
                        .create_page(&tab.id, &tab.window_id, &tab.url, tab.is_active)
                {
                    return Err(self.compensate_windows(error, &created_window_ids));
                }
            }
            return Ok(Response::Bool(true));
        }

        let window_id = self
            .windows
            .active_window_id()
            .ok_or_else(|| CoreError::Message("No startup window".into()))?
            .to_owned();
        self.adapter
            .create_window(&window_id, false)
            .map_err(|e| CoreError::Message(e.to_string()))?;
        let url = if behavior == "homePage" {
            SettingsService::get(&self.repository, "homeUrl")
                .map_err(|e| CoreError::Message(e.to_string()))?
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "https://example.com".into())
        } else {
            self.startup_url()
        };
        let tab = self.tabs.create_tab(window_id.clone(), url, true);
        self.windows.attach_tab(&window_id, &tab.id, true);
        self.adapter
            .create_page(&tab.id, &window_id, &tab.url, tab.is_active)
            .map_err(|e| CoreError::Message(e.to_string()))?;
        self.emit(Event::TabCreated(tab));
        self.mark_session_dirty();
        Ok(Response::Bool(true))
    }

    fn mark_session_dirty(&mut self) {
        self.session_dirty_since = Some(Instant::now());
    }

    pub fn flush_session(&mut self, force: bool) -> CoreResult<bool> {
        let Some(dirty_since) = self.session_dirty_since else {
            return Ok(false);
        };
        if !force && dirty_since.elapsed() < Duration::from_millis(500) {
            return Ok(false);
        }
        let private_windows: std::collections::HashSet<String> = self
            .windows
            .list()
            .into_iter()
            .filter(|window| window.is_private)
            .map(|window| window.id)
            .collect();
        let windows: Vec<_> = self
            .windows
            .list()
            .into_iter()
            .filter(|window| !window.is_private)
            .collect();
        let active_window_id = self
            .windows
            .active_window_id()
            .filter(|id| windows.iter().any(|window| &window.id == id))
            .map(ToOwned::to_owned)
            .or_else(|| windows.last().map(|window| window.id.clone()));
        let session = SessionState {
            active_window_id,
            windows,
            tabs: self
                .tabs
                .list()
                .into_iter()
                .filter(|tab| !private_windows.contains(&tab.window_id))
                .collect(),
        };
        let json =
            serde_json::to_string(&session).map_err(|e| CoreError::Message(e.to_string()))?;
        self.repository
            .set_session(&json)
            .map_err(|e| CoreError::Message(e.to_string()))?;
        self.session_dirty_since = None;
        Ok(true)
    }

    fn build_settings_snapshot(&self) -> serde_json::Value {
        let keys = [
            "homepage",
            "startupBehavior",
            "searchEngine",
            "customSearchUrl",
            "theme",
            "appearance",
            "sidebarVisible",
            "sidebarWidth",
            "newTabPage",
            "homeUrl",
            "language",
            "defaultZoomLevel",
            "downloadDirectory",
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

    /// Returns the startup URL for a new tab based on the newTabPage and homeUrl settings.
    fn startup_url(&self) -> String {
        let new_tab_page = SettingsService::get(&self.repository, "newTabPage")
            .ok()
            .flatten()
            .unwrap_or_else(|| "blank".into());
        let home_url = SettingsService::get(&self.repository, "homeUrl")
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
            .or_else(|| {
                SettingsService::get(&self.repository, "homepage")
                    .ok()
                    .flatten()
            })
            .unwrap_or_else(|| "fubuki://newtab/".into());
        if new_tab_page == "home" {
            home_url
        } else {
            "fubuki://newtab/".into()
        }
    }

    pub(crate) fn emit(&mut self, event: Event) {
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

    fn clear_browsing_data(&mut self, target: &str) -> CoreResult<()> {
        let all = target == "all";
        let clear_bookmarks = all || target == "bookmarks";
        let clear_history = all || target == "history";
        let clear_downloads = all || target == "downloads";
        let clear_permissions = all || target == "permissions";
        let clear_logs = all || target == "logs";
        if !(clear_bookmarks || clear_history || clear_downloads || clear_permissions || clear_logs)
        {
            return Err(CoreError::Message(format!(
                "unknown data clear target: {target}"
            )));
        }

        let bookmarks = clear_bookmarks
            .then(|| self.repository.list_bookmarks())
            .transpose()
            .map_err(|e| CoreError::Message(e.to_string()))?;
        let history = clear_history
            .then(|| self.repository.list_history())
            .transpose()
            .map_err(|e| CoreError::Message(e.to_string()))?;
        let downloads = clear_downloads
            .then(|| self.repository.list_downloads())
            .transpose()
            .map_err(|e| CoreError::Message(e.to_string()))?;
        let permissions = clear_permissions
            .then(|| self.repository.list_permissions())
            .transpose()
            .map_err(|e| CoreError::Message(e.to_string()))?;
        let logs = clear_logs
            .then(|| self.repository.list_logs(300))
            .transpose()
            .map_err(|e| CoreError::Message(e.to_string()))?;

        let result = (|| -> Result<(), String> {
            if clear_bookmarks {
                self.repository
                    .clear_bookmarks()
                    .map_err(|e| e.to_string())?;
            }
            if clear_history {
                HistoryService::clear_range(&self.repository, "all").map_err(|e| e.to_string())?;
            }
            if clear_downloads {
                self.repository
                    .clear_downloads()
                    .map_err(|e| e.to_string())?;
            }
            if clear_permissions {
                for permission in permissions.as_deref().unwrap_or_default() {
                    self.repository
                        .remove_permission(&permission.origin, &permission.permission)
                        .map_err(|e| e.to_string())?;
                }
            }
            if clear_logs {
                self.repository.clear_logs().map_err(|e| e.to_string())?;
            }
            Ok(())
        })();
        if let Err(error) = result {
            let mut compensation = Vec::new();
            if let Some(records) = bookmarks.as_deref() {
                for record in records {
                    if let Err(e) = self.repository.save_bookmark(
                        &record.title,
                        &record.url,
                        &record.favicon_url,
                    ) {
                        compensation.push(e.to_string());
                    }
                }
            }
            if let Some(records) = history.as_deref() {
                for record in records {
                    if let Err(e) =
                        self.repository
                            .add_history(&record.title, &record.url, &record.favicon_url)
                    {
                        compensation.push(e.to_string());
                    }
                }
            }
            if let Some(records) = downloads.as_deref() {
                for record in records {
                    if let Err(e) = self.repository.upsert_download(
                        &record.url,
                        &record.path,
                        &record.state,
                        record.percent,
                    ) {
                        compensation.push(e.to_string());
                    }
                }
            }
            if let Some(records) = permissions.as_deref() {
                for record in records {
                    if let Err(e) = self.repository.set_permission(
                        &record.origin,
                        &record.permission,
                        &record.value,
                    ) {
                        compensation.push(e.to_string());
                    }
                }
            }
            if let Some(records) = logs.as_deref() {
                for record in records.iter().rev() {
                    if let Err(e) = self.repository.add_log(&record.level, &record.message) {
                        compensation.push(e.to_string());
                    }
                }
            }
            return Err(Self::compensation_error(
                EngineError::Message(error),
                compensation,
            ));
        }

        if clear_bookmarks {
            self.emit(Event::BookmarkChanged { url: String::new() });
        }
        if clear_history {
            self.emit(Event::HistoryChanged { url: None });
        }
        if clear_downloads {
            self.emit(Event::DownloadChanged {
                url: None,
                path: None,
            });
        }
        if clear_permissions {
            self.emit(Event::PermissionChanged {
                origin: String::new(),
                permission: String::new(),
            });
        }
        Ok(())
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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionState {
    active_window_id: Option<String>,
    windows: Vec<frost_protocol::WindowState>,
    tabs: Vec<frost_protocol::TabState>,
}

#[derive(Clone)]
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
    session: std::cell::RefCell<Option<String>>,
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

    fn remove_setting(&self, key: &str) -> frost_store::StoreResult<()> {
        self.values.borrow_mut().remove(key);
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
        Ok(self.session.borrow().clone())
    }

    fn set_session(&self, json: &str) -> frost_store::StoreResult<()> {
        *self.session.borrow_mut() = Some(json.to_owned());
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
    use frost_engine_api::{EngineAdapter, EngineResult};
    use frost_protocol::{
        ExternalCapability, ExternalCommand, ExternalCommandEnvelope, HostCommand, HostEvent,
        HostEventEnvelope, Request, Response,
    };

    use super::*;

    fn result_adapter(
        ok: bool,
        observed: Arc<std::sync::Mutex<Vec<HostCommand>>>,
    ) -> HostCommandAdapter {
        let (command_tx, command_rx) = crossbeam_channel::unbounded::<HostCommandEnvelope>();
        let (result_tx, result_rx) = crossbeam_channel::unbounded::<HostCommandResultEnvelope>();
        let notify = Arc::new(move || {
            if let Ok(envelope) = command_rx.try_recv() {
                observed.lock().unwrap().push(envelope.command);
                result_tx
                    .send(HostCommandResultEnvelope {
                        version: frost_protocol::PROTOCOL_VERSION,
                        command_id: envelope.id,
                        ok,
                        error: (!ok).then(|| "injected native failure".into()),
                    })
                    .unwrap();
            }
        });
        HostCommandAdapter::with_results(command_tx, result_rx, notify)
    }

    fn sequenced_result_adapter(
        results: Arc<std::sync::Mutex<std::collections::VecDeque<bool>>>,
        observed: Arc<std::sync::Mutex<Vec<HostCommand>>>,
    ) -> HostCommandAdapter {
        let (command_tx, command_rx) = crossbeam_channel::unbounded::<HostCommandEnvelope>();
        let (result_tx, result_rx) = crossbeam_channel::unbounded::<HostCommandResultEnvelope>();
        let notify = Arc::new(move || {
            if let Ok(envelope) = command_rx.try_recv() {
                observed.lock().unwrap().push(envelope.command);
                let ok = results.lock().unwrap().pop_front().unwrap_or(true);
                result_tx
                    .send(HostCommandResultEnvelope {
                        version: frost_protocol::PROTOCOL_VERSION,
                        command_id: envelope.id,
                        ok,
                        error: (!ok).then(|| "injected native failure".into()),
                    })
                    .unwrap();
            }
        });
        HostCommandAdapter::with_results(command_tx, result_rx, notify)
    }

    #[derive(Default)]
    struct FailSecondCreateAdapter {
        create_count: usize,
    }

    impl EngineAdapter for FailSecondCreateAdapter {
        fn create_page(&mut self, _: &str, _: &str, _: &str, _: bool) -> EngineResult<()> {
            self.create_count += 1;
            if self.create_count == 2 {
                return Err(EngineError::Message("replacement rejected".into()));
            }
            Ok(())
        }

        fn close_page(&mut self, _: &str, _: Option<&str>, _: Option<&str>) -> EngineResult<()> {
            Ok(())
        }

        fn activate_page(&mut self, _: &str) -> EngineResult<()> {
            Ok(())
        }

        fn pin_page(&mut self, _: &str, _: bool) -> EngineResult<()> {
            Ok(())
        }

        fn move_page(&mut self, _: &str, _: usize) -> EngineResult<()> {
            Ok(())
        }

        fn set_overlay(
            &mut self,
            _: &str,
            _: bool,
            _: Option<f64>,
            _: Option<f64>,
        ) -> EngineResult<()> {
            Ok(())
        }

        fn navigate(&mut self, _: &str, _: &str) -> EngineResult<()> {
            Ok(())
        }

        fn reload(&mut self, _: &str) -> EngineResult<()> {
            Ok(())
        }

        fn stop(&mut self, _: &str) -> EngineResult<()> {
            Ok(())
        }

        fn go_back(&mut self, _: &str) -> EngineResult<()> {
            Ok(())
        }

        fn go_forward(&mut self, _: &str) -> EngineResult<()> {
            Ok(())
        }

        fn create_window(&mut self, _: &str, _: bool) -> EngineResult<()> {
            Ok(())
        }

        fn close_window(&mut self, _: &str) -> EngineResult<()> {
            Ok(())
        }

        fn apply_setting(&mut self, _: &str, _: &str) -> EngineResult<()> {
            Ok(())
        }
    }

    #[test]
    fn creates_and_activates_tabs() {
        let mut core = BrowserCore::new();

        let response = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
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
    fn closing_the_last_tab_creates_an_engine_owned_replacement() {
        let mut core = BrowserCore::new();
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: None,
            active: true,
        }));
        let tab_id = core.snapshot().tabs[0].id.clone();

        let response = core.process(ProtocolRequest::new(Request::TabsClose {
            tab_id: tab_id.clone(),
        }));

        assert!(matches!(response.response, Response::Bool(true)));
        let state = core.snapshot();
        assert_eq!(state.tabs.len(), 1);
        assert!(state.tabs[0].is_active);
        assert_eq!(state.tabs[0].url, "fubuki://newtab/");
    }

    #[test]
    fn failed_last_tab_replacement_restores_original_engine_state() {
        let mut core = BrowserCore::with_adapter_and_settings(
            FailSecondCreateAdapter::default(),
            InMemoryStore::default(),
        );
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: None,
            active: true,
        }));
        let tab_id = core.snapshot().tabs[0].id.clone();

        let response = core.process(ProtocolRequest::new(Request::TabsClose {
            tab_id: tab_id.clone(),
        }));

        assert!(!response.ok);
        let state = core.snapshot();
        assert_eq!(state.tabs.len(), 1);
        assert_eq!(state.tabs[0].id, tab_id);
        assert!(state.tabs[0].is_active);
        assert_eq!(state.windows[0].tab_ids, vec![tab_id]);
    }

    #[test]
    fn emits_differential_events() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::new();
        core.set_event_sender(tx);

        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: None,
            active: true,
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
        }));

        assert_eq!(response.response, Response::Bool(true));
        let command = host_rx.try_recv().unwrap();
        assert!(matches!(
            command.command,
            HostCommand::PageCreate { url, active: true, .. } if url == "https://example.com"
        ));
    }

    #[test]
    fn host_command_adapter_retains_out_of_order_results() {
        let (command_tx, command_rx) = crossbeam_channel::unbounded::<HostCommandEnvelope>();
        let (result_tx, result_rx) = crossbeam_channel::unbounded::<HostCommandResultEnvelope>();
        let notify = Arc::new(move || {
            let command = command_rx.try_recv().expect("host command");
            result_tx
                .send(HostCommandResultEnvelope {
                    version: frost_protocol::PROTOCOL_VERSION,
                    command_id: "later-command".into(),
                    ok: true,
                    error: None,
                })
                .unwrap();
            result_tx
                .send(HostCommandResultEnvelope {
                    version: frost_protocol::PROTOCOL_VERSION,
                    command_id: command.id,
                    ok: true,
                    error: None,
                })
                .unwrap();
        });
        let mut adapter = HostCommandAdapter::with_results(command_tx, result_rx, notify);

        adapter
            .create_page("tab-1", "window-1", "about:blank", true)
            .unwrap();
        assert!(
            adapter
                .pending_results
                .lock()
                .unwrap()
                .contains_key("later-command")
        );
    }

    #[test]
    fn background_and_duplicate_tabs_stay_inactive_in_host_commands() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://active.example".into()),
            active: true,
        }));
        let active_id = core.snapshot().tabs[0].id.clone();
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://background.example".into()),
            active: false,
        }));
        assert_eq!(
            core.snapshot().windows[0].active_tab_id,
            Some(active_id.clone())
        );
        core.process(ProtocolRequest::new(Request::TabsDuplicate {
            tab_id: active_id.clone(),
        }));

        let snapshot = core.snapshot();
        assert_eq!(snapshot.windows[0].active_tab_id, Some(active_id.clone()));
        assert_eq!(snapshot.tabs.iter().filter(|tab| tab.is_active).count(), 1);

        let commands: Vec<_> = host_rx
            .try_iter()
            .map(|envelope| envelope.command)
            .collect();
        assert!(matches!(
            &commands[1],
            HostCommand::PageCreate { active: false, url, .. }
                if url == "https://background.example"
        ));
        assert!(matches!(
            &commands[2],
            HostCommand::PageCreate { active: false, url, .. }
                if url == "https://active.example"
        ));
    }

    #[test]
    fn session_restore_page_commands_preserve_each_tabs_activity() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut adapter = HostCommandAdapter::new(host_tx);

        // Session restoration replays persisted tabs through the adapter with
        // each tab's persisted activity rather than inferring it from order.
        adapter
            .create_page("tab-active", "window-1", "https://active.example", true)
            .unwrap();
        adapter
            .create_page(
                "tab-background",
                "window-1",
                "https://background.example",
                false,
            )
            .unwrap();

        let commands: Vec<_> = host_rx
            .try_iter()
            .map(|envelope| envelope.command)
            .collect();
        assert!(matches!(
            &commands[0],
            HostCommand::PageCreate {
                tab_id,
                active: true,
                ..
            } if tab_id == "tab-active"
        ));
        assert!(matches!(
            &commands[1],
            HostCommand::PageCreate {
                tab_id,
                active: false,
                ..
            } if tab_id == "tab-background"
        ));
    }

    #[test]
    fn closing_active_tab_sends_engine_selected_successor() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );
        for url in ["a", "b", "c"] {
            core.process(ProtocolRequest::new(Request::TabsCreate {
                url: Some(url.into()),
                active: true,
            }));
        }
        let tabs = core.snapshot().tabs;
        core.process(ProtocolRequest::new(Request::TabsActivate {
            tab_id: tabs[1].id.clone(),
        }));
        host_rx.try_iter().for_each(drop);

        core.process(ProtocolRequest::new(Request::TabsClose {
            tab_id: tabs[1].id.clone(),
        }));

        let command = host_rx.try_recv().unwrap().command;
        assert_eq!(
            command,
            HostCommand::PageClose {
                tab_id: tabs[1].id.clone(),
                window_id: Some(tabs[1].window_id.clone()),
                successor_tab_id: Some(tabs[2].id.clone()),
            }
        );
        let snapshot = core.snapshot();
        assert!(
            snapshot
                .tabs
                .iter()
                .any(|tab| tab.id == tabs[2].id && tab.is_active)
        );
        assert_eq!(snapshot.windows[0].active_tab_id, Some(tabs[2].id.clone()));
    }

    #[test]
    fn tab_presentation_changes_are_engine_to_host_commands() {
        let (host_tx, host_rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::with_adapter_and_settings(
            HostCommandAdapter::new(host_tx),
            InMemoryStore::default(),
        );
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
        }));
        let tab_id = core.snapshot().tabs[0].id.clone();
        let _ = host_rx.try_recv().unwrap();
        core.process(ProtocolRequest::new(Request::TabsActivate {
            tab_id: tab_id.clone(),
        }));
        core.process(ProtocolRequest::new(Request::TabsPin {
            tab_id: tab_id.clone(),
            pinned: true,
        }));
        core.process(ProtocolRequest::new(Request::TabsMove {
            tab_id: tab_id.clone(),
            to_index: 0,
        }));
        assert!(
            matches!(host_rx.try_recv().unwrap().command, HostCommand::PageActivate { tab_id: id } if id == tab_id)
        );
        assert!(
            matches!(host_rx.try_recv().unwrap().command, HostCommand::PagePin { tab_id: id, pinned: true } if id == tab_id)
        );
        assert!(
            matches!(host_rx.try_recv().unwrap().command, HostCommand::PageMove { tab_id: id, to_index: 0 } if id == tab_id)
        );
    }

    #[test]
    fn applies_host_page_events_to_core_state() {
        let mut core = BrowserCore::new();
        let create = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://example.com".into()),
            active: true,
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
    /// WindowsCreate owns a startup tab, so keep it and add the remaining tabs.
    fn create_window_with_tabs(core: &mut BrowserCore, count: usize) -> (String, Vec<String>) {
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
        for i in 1..count {
            let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
                url: Some(format!("https://example{}.com", i)),
                active: true,
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
        tab_ids.truncate(count);
        (window_id, tab_ids)
    }

    #[test]
    fn close_other_tabs_scoped_to_window() {
        let mut core = BrowserCore::new();
        // Window 1 (default) – 1 tab
        let resp = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://w1.com".into()),
            active: true,
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
    fn native_window_focus_and_close_events_update_engine_state() {
        let mut core = BrowserCore::new();
        let first_window = core.snapshot().windows[0].id.clone();
        let (second_window, _) = create_window_with_tabs(&mut core, 1);
        assert_eq!(
            core.snapshot().active_window_id.as_deref(),
            Some(second_window.as_str())
        );

        core.process_host_event(HostEventEnvelope::new(HostEvent::WindowFocused {
            window_id: first_window.clone(),
        }))
        .unwrap();
        assert_eq!(
            core.snapshot().active_window_id.as_deref(),
            Some(first_window.as_str())
        );

        core.process_host_event(HostEventEnvelope::new(HostEvent::WindowClosed {
            window_id: second_window.clone(),
        }))
        .unwrap();
        let mut snapshot = core.snapshot();
        assert!(!snapshot.windows.iter().any(|w| w.id == second_window));
        assert!(!snapshot.tabs.iter().any(|t| t.window_id == second_window));

        assert!(
            core.process(ProtocolRequest::new(Request::WindowsReopenClosed))
                .ok
        );
        snapshot = core.snapshot();
        assert!(snapshot.windows.iter().any(|w| w.id == second_window));
        assert!(snapshot.tabs.iter().any(|t| t.window_id == second_window));
    }

    #[test]
    fn private_windows_are_not_kept_in_the_reopen_history() {
        let mut core = BrowserCore::new();
        assert!(
            core.process(ProtocolRequest::new(Request::WindowsCreatePrivate))
                .ok
        );
        let private_window = core.snapshot().active_window_id.unwrap();
        assert!(
            core.process(ProtocolRequest::new(Request::WindowsClose {
                window_id: Some(private_window),
            }))
            .ok
        );
        assert_eq!(
            core.process(ProtocolRequest::new(Request::WindowsReopenClosed))
                .response,
            Response::Bool(false)
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

    #[test]
    fn settings_page_keys_are_supported_by_the_engine() {
        let mut core = BrowserCore::new();
        for (key, value) in [
            ("startupBehavior", "restore"),
            ("downloadDirectory", "/tmp/downloads"),
            ("askBeforeDownload", "on"),
        ] {
            let response = core.process(ProtocolRequest::new(Request::SettingsSet {
                key: key.into(),
                value: value.into(),
            }));
            assert_eq!(response.response, Response::Bool(true), "{key}");
        }
        let Response::AppSnapshot(snapshot) = core
            .process(ProtocolRequest::new(Request::AppSnapshot))
            .response
        else {
            panic!("expected snapshot");
        };
        assert_eq!(snapshot.settings["startupBehavior"], "restore");
        assert_eq!(snapshot.settings["downloadDirectory"], "/tmp/downloads");
        assert_eq!(snapshot.settings["askBeforeDownload"], "on");
    }

    #[test]
    fn close_other_and_close_right_reject_unknown_tabs() {
        let mut core = BrowserCore::new();
        for request in [
            Request::TabsCloseOther {
                tab_id: "missing".into(),
            },
            Request::TabsCloseToRight {
                tab_id: "missing".into(),
            },
        ] {
            let response = core.process(ProtocolRequest::new(request));
            assert_eq!(response.response, Response::Bool(false));
        }
    }

    #[test]
    fn command_close_other_uses_the_active_tab_when_no_id_is_given() {
        let mut core = BrowserCore::new();
        for index in 0..3 {
            assert!(
                core.process(ProtocolRequest::new(Request::TabsCreate {
                    url: Some(format!("https://command-{index}.example/")),
                    active: true,
                }))
                .ok
            );
        }
        let active = core.tabs.active_tab().unwrap().id;
        let response = core.process(ProtocolRequest::new(Request::CommandsExecute {
            id: "tabs.closeOther".into(),
            args: None,
        }));
        assert_eq!(response.response, Response::Bool(true));
        assert_eq!(core.tabs.list().len(), 1);
        assert_eq!(core.tabs.list()[0].id, active);
    }

    #[test]
    fn moving_a_tab_forward_by_one_does_not_skip_the_target_slot() {
        let mut core = BrowserCore::new();
        for index in 0..3 {
            assert!(
                core.process(ProtocolRequest::new(Request::TabsCreate {
                    url: Some(format!("https://move-{index}.example/")),
                    active: true,
                }))
                .ok
            );
        }
        let before = core.tabs.list();
        assert!(
            core.process(ProtocolRequest::new(Request::TabsMove {
                tab_id: before[0].id.clone(),
                to_index: 1,
            }))
            .ok
        );
        let after = core.tabs.list();
        assert_eq!(after[0].id, before[1].id);
        assert_eq!(after[1].id, before[0].id);
        assert_eq!(after[2].id, before[2].id);
    }

    #[test]
    fn native_failure_does_not_leave_ghost_tab_or_event() {
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            result_adapter(false, observed),
            InMemoryStore::default(),
        );
        let before = core.snapshot();
        let response = core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://failure.example".into()),
            active: true,
        }));
        assert!(!response.ok);
        assert_eq!(core.snapshot().tabs, before.tabs);
        assert_eq!(core.snapshot().windows, before.windows);
        assert!(core.recent_events().is_empty());
    }

    #[test]
    fn setting_is_applied_to_native_before_it_is_saved_and_announced() {
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            result_adapter(true, observed.clone()),
            InMemoryStore::default(),
        );
        let response = core.process(ProtocolRequest::new(Request::SettingsSet {
            key: "sidebarWidth".into(),
            value: "240".into(),
        }));
        assert!(response.ok);
        assert!(matches!(
            observed.lock().unwrap().as_slice(),
            [HostCommand::SettingsApply { key, value }]
                if key == "sidebarWidth" && value == "240"
        ));
        assert_eq!(
            SettingsService::get(&core.repository, "sidebarWidth").unwrap(),
            Some("240".into())
        );
        assert!(matches!(
            core.recent_events().last().map(|event| &event.event),
            Some(Event::SettingChanged(change)) if change.value == "240"
        ));
    }

    #[test]
    fn failed_setting_apply_is_not_saved_or_announced() {
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            result_adapter(false, observed),
            InMemoryStore::default(),
        );
        let response = core.process(ProtocolRequest::new(Request::SettingsSet {
            key: "sidebarVisible".into(),
            value: "hide".into(),
        }));
        assert!(!response.ok);
        assert_eq!(
            SettingsService::get(&core.repository, "sidebarVisible").unwrap(),
            None
        );
        assert!(core.recent_events().is_empty());
    }

    #[test]
    fn startup_behavior_selects_new_tab_home_page_and_restore() {
        for (behavior, expected_url) in [
            ("newTab", "fubuki://newtab/"),
            ("homePage", "https://home.example/"),
        ] {
            let store = InMemoryStore::default();
            SettingsService::set(&store, "startupBehavior", behavior).unwrap();
            SettingsService::set(&store, "homeUrl", "https://home.example/").unwrap();
            let mut core = BrowserCore::with_adapter_and_settings(NoopEngineAdapter, store);
            assert!(core.process(ProtocolRequest::new(Request::AppStartup)).ok);
            assert_eq!(core.snapshot().tabs[0].url, expected_url);
        }

        let store = InMemoryStore::default();
        SettingsService::set(&store, "startupBehavior", "restore").unwrap();
        let session = SessionState {
            active_window_id: Some("restored-window".into()),
            windows: vec![frost_protocol::WindowState {
                id: "restored-window".into(),
                active_tab_id: Some("restored-tab".into()),
                is_private: false,
                tab_ids: vec!["restored-tab".into()],
            }],
            tabs: vec![frost_protocol::TabState {
                id: "restored-tab".into(),
                window_id: "restored-window".into(),
                title: "Restored".into(),
                url: "https://restored.example/".into(),
                favicon_url: String::new(),
                error_text: String::new(),
                zoom_level: 0.0,
                is_loading: false,
                can_go_back: false,
                can_go_forward: false,
                is_active: true,
                is_pinned: false,
            }],
        };
        store
            .set_session(&serde_json::to_string(&session).unwrap())
            .unwrap();
        let mut core = BrowserCore::with_adapter_and_settings(NoopEngineAdapter, store);
        assert!(core.process(ProtocolRequest::new(Request::AppStartup)).ok);
        assert_eq!(core.snapshot().tabs[0].id, "restored-tab");
        assert_eq!(core.snapshot().tabs[0].url, "https://restored.example/");
    }

    #[test]
    fn repeated_tab_and_setting_operations_keep_state_consistent() {
        let mut core = BrowserCore::new();
        for index in 0..100 {
            assert!(
                core.process(ProtocolRequest::new(Request::TabsCreate {
                    url: Some(format!("https://stress-{index}.example/")),
                    active: true,
                }))
                .ok
            );
            let tab_id = core
                .snapshot()
                .active_window_id
                .as_deref()
                .and_then(|window_id| {
                    core.snapshot()
                        .windows
                        .into_iter()
                        .find(|window| window.id == window_id)
                })
                .and_then(|window| window.active_tab_id)
                .expect("active tab");
            assert!(
                core.process(ProtocolRequest::new(Request::TabsActivate {
                    tab_id: tab_id.clone(),
                }))
                .ok
            );
            assert!(
                core.process(ProtocolRequest::new(Request::SettingsSet {
                    key: "sidebarWidth".into(),
                    value: (196 + index % 50).to_string(),
                }))
                .ok
            );
            assert!(
                core.process(ProtocolRequest::new(Request::TabsClose { tab_id }))
                    .ok
            );

            let state = core.snapshot();
            for window in &state.windows {
                for tab_id in &window.tab_ids {
                    assert!(
                        state
                            .tabs
                            .iter()
                            .any(|tab| { tab.id == *tab_id && tab.window_id == window.id })
                    );
                }
                assert!(
                    window
                        .active_tab_id
                        .as_ref()
                        .is_none_or(|active| { window.tab_ids.contains(active) })
                );
            }
        }
    }

    #[test]
    fn failed_second_window_create_stage_rolls_back_host_and_engine() {
        let results = Arc::new(std::sync::Mutex::new(std::collections::VecDeque::from([
            true, false, true,
        ])));
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            sequenced_result_adapter(results, observed.clone()),
            InMemoryStore::default(),
        );
        let before = core.snapshot();

        let response = core.process(ProtocolRequest::new(Request::WindowsCreate));

        assert!(!response.ok);
        assert_eq!(core.snapshot(), before);
        assert!(matches!(
            observed.lock().unwrap().as_slice(),
            [
                HostCommand::WindowCreate { .. },
                HostCommand::PageCreate { .. },
                HostCommand::WindowClose { .. }
            ]
        ));
    }

    #[test]
    fn failed_private_window_page_and_compensation_reports_both_failures() {
        let results = Arc::new(std::sync::Mutex::new(std::collections::VecDeque::from([
            true, false, false,
        ])));
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            sequenced_result_adapter(results, observed.clone()),
            InMemoryStore::default(),
        );
        let before = core.snapshot();

        let response = core.process(ProtocolRequest::new(Request::WindowsCreatePrivate));

        assert!(!response.ok);
        assert!(matches!(
            response.response,
            Response::Error(message) if message.contains("compensation also failed")
        ));
        assert_eq!(core.snapshot(), before);
        assert!(matches!(
            observed.lock().unwrap().as_slice(),
            [
                HostCommand::WindowCreate { .. },
                HostCommand::PageCreate { .. },
                HostCommand::WindowClose { .. }
            ]
        ));
    }

    #[test]
    fn failed_move_to_new_window_stage_rolls_back_host_and_engine() {
        let results = Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            sequenced_result_adapter(results.clone(), observed.clone()),
            InMemoryStore::default(),
        );
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://move.example".into()),
            active: true,
        }));
        let tab_id = core.snapshot().tabs[0].id.clone();
        let before = core.snapshot();
        observed.lock().unwrap().clear();
        results
            .lock()
            .unwrap()
            .extend([true, true, false, true, true]);

        let response = core.process(ProtocolRequest::new(Request::TabsMoveToNewWindow {
            tab_id,
        }));

        assert!(!response.ok);
        assert_eq!(core.snapshot(), before);
        assert!(matches!(
            observed.lock().unwrap().as_slice(),
            [
                HostCommand::WindowCreate { .. },
                HostCommand::PageCreate { .. },
                HostCommand::PageClose { .. },
                HostCommand::PageClose { .. },
                HostCommand::WindowClose { .. }
            ]
        ));
    }

    #[test]
    fn partial_multi_close_recreates_pages_closed_before_failure() {
        let results = Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            sequenced_result_adapter(results.clone(), observed.clone()),
            InMemoryStore::default(),
        );
        for url in ["a", "b", "c"] {
            core.process(ProtocolRequest::new(Request::TabsCreate {
                url: Some(url.into()),
                active: url == "a",
            }));
        }
        let target = core.snapshot().tabs[0].id.clone();
        let before = core.snapshot();
        observed.lock().unwrap().clear();
        results.lock().unwrap().extend([true, false, true]);

        let response = core.process(ProtocolRequest::new(Request::TabsCloseOther {
            tab_id: target,
        }));

        assert!(!response.ok);
        assert_eq!(core.snapshot(), before);
        assert!(matches!(
            observed.lock().unwrap().as_slice(),
            [
                HostCommand::PageClose { .. },
                HostCommand::PageClose { .. },
                HostCommand::PageCreate { url, active: false, .. }
            ] if url == "b"
        ));
    }

    #[test]
    fn failed_reopen_window_page_stage_keeps_window_closed() {
        let results = Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            sequenced_result_adapter(results.clone(), observed.clone()),
            InMemoryStore::default(),
        );
        core.process(ProtocolRequest::new(Request::WindowsCreate));
        let window_id = core.snapshot().active_window_id.unwrap();
        core.process(ProtocolRequest::new(Request::WindowsClose {
            window_id: Some(window_id),
        }));
        let before = core.snapshot();
        observed.lock().unwrap().clear();
        results.lock().unwrap().extend([true, false, true]);

        let response = core.process(ProtocolRequest::new(Request::WindowsReopenClosed));

        assert!(!response.ok);
        assert_eq!(core.snapshot(), before);
        assert!(matches!(
            observed.lock().unwrap().as_slice(),
            [
                HostCommand::WindowCreate { .. },
                HostCommand::PageCreate { .. },
                HostCommand::WindowClose { .. }
            ]
        ));
    }

    #[test]
    fn failed_startup_restore_closes_every_created_native_window() {
        let store = InMemoryStore::default();
        SettingsService::set(&store, "startupBehavior", "restore").unwrap();
        let tab = |id: &str, window_id: &str| frost_protocol::TabState {
            id: id.into(),
            window_id: window_id.into(),
            title: id.into(),
            url: format!("https://{id}.example"),
            favicon_url: String::new(),
            error_text: String::new(),
            zoom_level: 0.0,
            is_loading: false,
            can_go_back: false,
            can_go_forward: false,
            is_active: true,
            is_pinned: false,
        };
        let session = SessionState {
            active_window_id: Some("window-2".into()),
            windows: vec![
                frost_protocol::WindowState {
                    id: "window-1".into(),
                    active_tab_id: Some("tab-1".into()),
                    is_private: false,
                    tab_ids: vec!["tab-1".into()],
                },
                frost_protocol::WindowState {
                    id: "window-2".into(),
                    active_tab_id: Some("tab-2".into()),
                    is_private: false,
                    tab_ids: vec!["tab-2".into()],
                },
            ],
            tabs: vec![tab("tab-1", "window-1"), tab("tab-2", "window-2")],
        };
        store
            .set_session(&serde_json::to_string(&session).unwrap())
            .unwrap();
        let results = Arc::new(std::sync::Mutex::new(std::collections::VecDeque::from([
            true, true, true, false, true, true,
        ])));
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            sequenced_result_adapter(results, observed.clone()),
            store,
        );
        let before = core.snapshot();

        let response = core.process(ProtocolRequest::new(Request::AppStartup));

        assert!(!response.ok);
        assert_eq!(core.snapshot(), before);
        assert!(matches!(
            observed.lock().unwrap().as_slice(),
            [
                HostCommand::WindowCreate { window_id: first, .. },
                HostCommand::WindowCreate { window_id: second, .. },
                HostCommand::PageCreate { .. },
                HostCommand::PageCreate { .. },
                HostCommand::WindowClose { window_id: close_second },
                HostCommand::WindowClose { window_id: close_first }
            ] if first == "window-1" && second == "window-2"
                && close_second == "window-2" && close_first == "window-1"
        ));
    }

    #[test]
    fn failed_closed_tab_restore_remains_retryable() {
        let results = Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            sequenced_result_adapter(results.clone(), observed),
            InMemoryStore::default(),
        );
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://closed.example".into()),
            active: true,
        }));
        let tab_id = core.snapshot().tabs[0].id.clone();
        core.process(ProtocolRequest::new(Request::TabsClose { tab_id }));
        let before = core.snapshot();
        results.lock().unwrap().push_back(false);

        let failed = core.process(ProtocolRequest::new(Request::TabsReopenClosed));
        assert!(!failed.ok);
        assert_eq!(core.snapshot(), before);

        let retry = core.process(ProtocolRequest::new(Request::TabsReopenClosed));
        assert!(retry.ok);
        assert!(
            core.snapshot()
                .tabs
                .iter()
                .any(|tab| tab.url == "https://closed.example")
        );
    }

    #[test]
    fn failed_last_tab_replacement_recreates_original_native_page() {
        let results = Arc::new(std::sync::Mutex::new(std::collections::VecDeque::new()));
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core = BrowserCore::with_adapter_and_settings(
            sequenced_result_adapter(results.clone(), observed.clone()),
            InMemoryStore::default(),
        );
        core.process(ProtocolRequest::new(Request::TabsCreate {
            url: Some("https://original.example".into()),
            active: true,
        }));
        let tab_id = core.snapshot().tabs[0].id.clone();
        observed.lock().unwrap().clear();
        results.lock().unwrap().extend([true, false, true]);

        let response = core.process(ProtocolRequest::new(Request::TabsClose {
            tab_id: tab_id.clone(),
        }));

        assert!(!response.ok);
        assert_eq!(core.snapshot().tabs[0].id, tab_id);
        let commands = observed.lock().unwrap();
        assert!(matches!(
            commands.as_slice(),
            [
                HostCommand::PageClose { .. },
                HostCommand::PageCreate { url: replacement, .. },
                HostCommand::PageCreate { url: original, active: true, .. }
            ] if replacement == "fubuki://newtab/" && original == "https://original.example"
        ));
    }

    #[test]
    fn failed_setting_apply_restores_previous_persisted_value() {
        let store = InMemoryStore::default();
        SettingsService::set(&store, "sidebarVisible", "show").unwrap();
        let observed = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut core =
            BrowserCore::with_adapter_and_settings(result_adapter(false, observed.clone()), store);

        let response = core.process(ProtocolRequest::new(Request::SettingsSet {
            key: "sidebarVisible".into(),
            value: "hide".into(),
        }));

        assert!(!response.ok);
        assert_eq!(
            SettingsService::get(&core.repository, "sidebarVisible").unwrap(),
            Some("show".into())
        );
        assert!(matches!(
            observed.lock().unwrap().as_slice(),
            [
                HostCommand::SettingsApply { value: next, .. },
                HostCommand::SettingsApply {
                    value: previous,
                    ..
                }
            ] if next == "hide" && previous == "show"
        ));
    }

    #[test]
    fn data_clear_all_removes_every_browsing_data_collection_and_emits_once() {
        let store = frost_store::SqliteStore::in_memory().unwrap();
        store
            .save_bookmark("bookmark", "https://bookmark.example", "")
            .unwrap();
        store
            .add_history("history", "https://history.example", "")
            .unwrap();
        store
            .upsert_download("https://download.example", "/tmp/file", "done", 100)
            .unwrap();
        store
            .set_permission("https://permission.example", "camera", "allow")
            .unwrap();
        store.add_log("info", "log entry").unwrap();
        let mut core = BrowserCore::with_adapter_and_settings(NoopEngineAdapter, store);

        let response = core.process(ProtocolRequest::new(Request::DataClear { target: None }));

        assert!(matches!(response.response, Response::Bool(true)));
        assert!(core.repository.list_bookmarks().unwrap().is_empty());
        assert!(core.repository.list_history().unwrap().is_empty());
        assert!(core.repository.list_downloads().unwrap().is_empty());
        assert!(core.repository.list_permissions().unwrap().is_empty());
        assert!(core.repository.list_logs(10).unwrap().is_empty());
        assert!(matches!(
            core.recent_events()[0].event,
            Event::BookmarkChanged { .. }
        ));
        assert_eq!(core.recent_events().len(), 4);
    }

    #[test]
    fn external_events_use_the_owned_event_history_and_delivery_path() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let mut core = BrowserCore::new();
        core.set_event_sender(tx);
        let mut policy = ExternalPolicy::new();

        let command = ExternalCommandEnvelope::new(
            "request-1",
            "automation",
            ExternalCapability::ReadState,
            ExternalCommand::StateRead,
        );
        let outcome = core.process_external(command, &mut policy);

        assert!(!outcome.allowed);
        assert!(matches!(
            core.recent_events().last().map(|event| &event.event),
            Some(Event::ExternalAudit { allowed: false, .. })
        ));
        assert!(matches!(
            rx.try_recv().unwrap().event,
            Event::ExternalAudit { allowed: false, .. }
        ));
    }

    #[test]
    fn private_active_window_is_not_persisted_in_session() {
        let store = frost_store::SqliteStore::in_memory().unwrap();
        let mut core = BrowserCore::with_adapter_and_settings(NoopEngineAdapter, store);
        core.process(ProtocolRequest::new(Request::WindowsCreatePrivate));
        core.flush_session(true).unwrap();

        let session = core.repository.get_session().unwrap().unwrap();
        let session: SessionState = serde_json::from_str(&session).unwrap();
        assert!(session.active_window_id.is_some());
        assert!(session.windows.iter().all(|window| !window.is_private));
        assert!(
            session
                .active_window_id
                .as_ref()
                .is_some_and(|id| session.windows.iter().any(|window| &window.id == id))
        );
    }
}
