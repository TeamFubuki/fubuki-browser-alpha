use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppState {
    pub protocol_version: u16,
    pub active_window_id: Option<String>,
    pub windows: Vec<WindowState>,
    pub tabs: Vec<TabState>,
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
