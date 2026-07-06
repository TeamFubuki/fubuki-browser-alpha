use serde::{Deserialize, Serialize};

use crate::state::{
    AppState, BookmarkRecord, DownloadRecord, HistoryRecord, TabState, WindowState,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolResponse {
    pub version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub ok: bool,
    #[serde(flatten)]
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "result", rename_all = "camelCase")]
pub enum Response {
    AppSnapshot(AppState),
    TabsList(Vec<TabState>),
    WindowsList(Vec<WindowState>),
    BookmarksList(Vec<BookmarkRecord>),
    HistoryList(Vec<HistoryRecord>),
    DownloadsList(Vec<DownloadRecord>),
    Setting(Option<String>),
    Bool(bool),
    Error(String),
}

impl ProtocolResponse {
    pub fn ok(id: Option<String>, response: Response) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            id,
            ok: true,
            response,
        }
    }

    pub fn error(id: Option<String>, message: impl Into<String>) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            id,
            ok: false,
            response: Response::Error(message.into()),
        }
    }
}
