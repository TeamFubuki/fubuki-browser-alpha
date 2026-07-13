use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub protocol_version: u16,
    pub active_window_id: Option<String>,
    pub windows: Vec<WindowState>,
    pub tabs: Vec<TabState>,
    #[serde(default)]
    pub history: Vec<HistoryRecord>,
    #[serde(default)]
    pub bookmarks: Vec<BookmarkRecord>,
    #[serde(default)]
    pub downloads: Vec<DownloadRecord>,
    #[serde(default)]
    pub permissions: Vec<PermissionRecord>,
    #[serde(default)]
    pub settings: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowState {
    pub id: String,
    pub active_tab_id: Option<String>,
    pub is_private: bool,
    pub tab_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabState {
    pub id: String,
    pub window_id: String,
    pub title: String,
    pub url: String,
    pub favicon_url: String,
    pub error_text: String,
    pub zoom_level: f64,
    pub is_loading: bool,
    pub can_go_back: bool,
    pub can_go_forward: bool,
    pub is_active: bool,
    pub is_pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    pub title: String,
    pub url: String,
    pub favicon_url: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookmarkRecord {
    pub title: String,
    pub url: String,
    pub favicon_url: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadRecord {
    #[serde(default)]
    pub download_id: String,
    pub url: String,
    pub path: String,
    pub state: String,
    pub percent: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRecord {
    pub origin: String,
    pub permission: String,
    pub value: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserCommand {
    pub id: String,
    pub title: String,
    pub category: String,
    pub shortcut: String,
}
