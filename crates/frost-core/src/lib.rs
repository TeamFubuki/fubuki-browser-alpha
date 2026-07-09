mod bookmark_service;
mod download_service;
mod external_router;
mod history_service;
mod settings_service;
mod tab_service;
mod window_service;

pub use external_router::{ExternalPolicy, ExternalResponse};

use std::time::{SystemTime, UNIX_EPOCH};

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
}

impl HostCommandAdapter {
    pub fn new(tx: Sender<HostCommandEnvelope>) -> Self {
        Self { tx }
    }

    fn send(&self, command: HostCommand) -> EngineResult<()> {
        let id = format!("host-command-{}", uuid::Uuid::new_v4());
        self.tx
            .send(HostCommandEnvelope::new(id, command))
            .map_err(|e| EngineError::Message(e.to_string()))
    }
}

impl EngineAdapter for HostCommandAdapter {
    fn create_page(&mut self, tab_id: &str, window_id: &str, url: &str) -> EngineResult<()> {
        self.send(HostCommand::PageCreate {
            tab_id: tab_id.to_owned(),
            window_id: window_id.to_owned(),
            url: url.to_owned(),
        })
    }

    fn close_page(&mut self, tab_id: &str) -> EngineResult<()> {
        self.send(HostCommand::PageClose {
            tab_id: tab_id.to_owned(),
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
                self.windows.attach_tab(&window_id, &tab.id, true);
                if let Err(e) = self.adapter.create_page(&tab.id, &window_id, &tab.url) {
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
                    self.emit(Event::TabActivated(TabActivated { tab_id }));
                }
                Ok(Response::Bool(activated))
            }
            Request::TabsClose { tab_id } => {
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
                    self.emit(Event::TabClosed(TabClosed { tab_id }));
                }
                Ok(Response::Bool(closed))
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
                let Some(tab) = self.tabs.duplicate_tab(&tab_id) else {
                    return Ok(Response::Bool(false));
                };
                self.windows.attach_tab(&tab.window_id, &tab.id, true);
                if let Err(e) = self.adapter.create_page(&tab.id, &tab.window_id, &tab.url) {
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
                if let Err(e) = self
                    .adapter
                    .create_page(&created.id, &window_id, &created.url)
                {
                    self.windows.detach_tab(&created.id);
                    self.tabs.remove_tab(&created.id);
                    return Err(CoreError::Message(e.to_string()));
                }
                self.emit(Event::TabCreated(created));
                Ok(Response::Bool(true))
            }
            Request::TabsCloseOther { tab_id } => {
                let closed = self.tabs.close_other_tabs(&tab_id);
                for tab in &closed {
                    self.windows.detach_tab(&tab.id);
                    self.emit(Event::TabClosed(TabClosed {
                        tab_id: tab.id.clone(),
                    }));
                }
                self.closed_tabs.extend(closed);
                self.trim_closed_tabs();
                Ok(Response::Bool(true))
            }
            Request::TabsCloseToRight { tab_id } => {
                let closed = self.tabs.close_tabs_to_right(&tab_id);
                for tab in &closed {
                    self.windows.detach_tab(&tab.id);
                    self.emit(Event::TabClosed(TabClosed {
                        tab_id: tab.id.clone(),
                    }));
                }
                self.closed_tabs.extend(closed);
                self.trim_closed_tabs();
                Ok(Response::Bool(true))
            }
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
            Request::TabsMoveToNewWindow { tab_id } => {
                if !self.tabs.contains(&tab_id) {
                    return Ok(Response::Bool(false));
                }
                let previous_window = self.tabs.get_tab(&tab_id).map(|tab| tab.window_id.clone());
                let window_id = self.windows.create_window(false);
                self.tabs.move_tab_to_window(&tab_id, &window_id);
                self.windows.move_tab_to_window(&tab_id, &window_id);
                if let Err(e) = self.adapter.create_window(&window_id, false) {
                    self.tabs
                        .move_tab_to_window(&tab_id, &previous_window.clone().unwrap_or_default());
                    self.windows
                        .move_tab_to_window(&tab_id, &previous_window.clone().unwrap_or_default());
                    self.windows.close_window(&window_id);
                    return Err(CoreError::Message(e.to_string()));
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
        }
    }

    pub fn process_host_event(&mut self, envelope: HostEventEnvelope) -> CoreResult<()> {
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
            return Ok(());
        }
        Err(CoreError::Message(format!(
            "Host command {} failed: {}",
            result.command_id,
            result.error.unwrap_or_else(|| "unknown error".into())
        )))
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
            HostCommand::PageCreate { url, .. } if url == "https://example.com"
        ));
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
