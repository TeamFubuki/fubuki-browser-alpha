mod bookmark_service;
mod download_service;
mod history_service;
mod settings_service;
mod tab_service;
mod window_service;

use crossbeam_channel::{Receiver, Sender};
use frost_engine_api::{EngineAdapter, NoopEngineAdapter};
use frost_protocol::{
    Event, EventEnvelope, ProtocolRequest, ProtocolResponse, Request, Response, SettingChanged,
    TabClosed, TabPatch,
};
use frost_store::{BookmarkRepository, DownloadRepository, HistoryRepository, SettingsRepository};
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

pub struct BrowserCore<A = NoopEngineAdapter, S = InMemoryStore> {
    adapter: A,
    repository: S,
    tabs: TabService,
    windows: WindowService,
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
    S: SettingsRepository + BookmarkRepository + HistoryRepository + DownloadRepository,
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
                self.windows.attach_tab(&window_id, &tab.id);
                self.adapter
                    .create_page(&tab.id, &window_id, &tab.url)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                self.emit(Event::TabCreated(tab));
                Ok(Response::Bool(true))
            }
            Request::TabsActivate { tab_id } => {
                let activated = self.tabs.activate_tab(&tab_id);
                if activated {
                    self.windows.set_active_tab(&tab_id);
                    self.emit(Event::TabActivated(TabClosed { tab_id }));
                }
                Ok(Response::Bool(activated))
            }
            Request::TabsClose { tab_id } => {
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
            Request::TabsGoBack { tab_id } => self.host_tab_action(&tab_id, HostTabAction::GoBack),
            Request::TabsGoForward { tab_id } => {
                self.host_tab_action(&tab_id, HostTabAction::GoForward)
            }
            Request::WindowsList => Ok(Response::WindowsList(self.windows.list())),
            Request::WindowsCreate => {
                let window_id = self.windows.create_window(false);
                self.adapter
                    .create_window(&window_id)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                if let Some(window) = self.windows.get_window(&window_id) {
                    self.emit(Event::WindowCreated(window));
                }
                Ok(Response::Bool(true))
            }
            Request::WindowsClose { window_id } => {
                let target = window_id
                    .or_else(|| self.windows.active_window_id().map(ToOwned::to_owned))
                    .ok_or_else(|| CoreError::Message("No active window".into()))?;
                let closed = self.windows.close_window(&target);
                if closed {
                    self.adapter
                        .close_window(&target)
                        .map_err(|e| CoreError::Message(e.to_string()))?;
                    self.emit(Event::WindowClosed { window_id: target });
                }
                Ok(Response::Bool(closed))
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
        frost_protocol::AppState {
            protocol_version: frost_protocol::PROTOCOL_VERSION,
            active_window_id: self.windows.active_window_id().map(ToOwned::to_owned),
            windows: self.windows.list(),
            tabs: self.tabs.list(),
            history: self.repository.list_history().unwrap_or_default(),
            bookmarks: self.repository.list_bookmarks().unwrap_or_default(),
            downloads: self.repository.list_downloads().unwrap_or_default(),
        }
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
        self.history
            .borrow_mut()
            .push(frost_protocol::HistoryRecord {
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
        if range == "all" {
            self.history.borrow_mut().clear();
            return Ok(true);
        }
        Ok(false)
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

#[cfg(test)]
mod tests {
    use frost_protocol::{Request, Response};

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
}
