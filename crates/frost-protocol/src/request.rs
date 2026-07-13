use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolRequest {
    #[serde(default = "default_version")]
    pub version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(flatten)]
    pub request: Request,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireProtocolRequest {
    #[serde(default = "default_version")]
    version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(flatten)]
    request: Request,
}

impl<'de> Deserialize<'de> for ProtocolRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut value = Value::deserialize(deserializer)?;
        if should_drop_empty_params(&value)
            && let Value::Object(object) = &mut value
        {
            object.remove("params");
        }
        let wire = WireProtocolRequest::deserialize(value).map_err(serde::de::Error::custom)?;
        Ok(Self {
            version: wire.version,
            id: wire.id,
            request: wire.request,
        })
    }
}

fn should_drop_empty_params(value: &Value) -> bool {
    let Value::Object(object) = value else {
        return false;
    };
    let Some(Value::String(method)) = object.get("method") else {
        return false;
    };
    let has_empty_params =
        matches!(object.get("params"), Some(Value::Object(params)) if params.is_empty());
    has_empty_params
        && matches!(
            method.as_str(),
            "app.snapshot"
                | "tabs.list"
                | "windows.list"
                | "windows.create"
                | "windows.createPrivate"
                | "windows.reopenClosed"
                | "bookmarks.list"
                | "history.list"
                | "downloads.list"
                | "logs.clear"
                | "commands.list"
                | "tabs.reopenClosed"
                | "tabs.home"
        )
}

fn default_version() -> u16 {
    crate::PROTOCOL_VERSION
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum Request {
    #[serde(rename = "app.snapshot")]
    AppSnapshot,
    #[serde(rename = "tabs.list")]
    TabsList,
    #[serde(rename = "tabs.create", rename_all = "camelCase")]
    TabsCreate { url: Option<String>, active: bool },
    #[serde(rename = "tabs.activate", rename_all = "camelCase")]
    TabsActivate { tab_id: String },
    #[serde(rename = "tabs.close", rename_all = "camelCase")]
    TabsClose { tab_id: String },
    #[serde(rename = "tabs.pin", rename_all = "camelCase")]
    TabsPin { tab_id: String, pinned: bool },
    #[serde(rename = "tabs.duplicate", rename_all = "camelCase")]
    TabsDuplicate { tab_id: String },
    #[serde(rename = "tabs.reopenClosed")]
    TabsReopenClosed,
    #[serde(rename = "tabs.closeOther", rename_all = "camelCase")]
    TabsCloseOther { tab_id: String },
    #[serde(rename = "tabs.closeToRight", rename_all = "camelCase")]
    TabsCloseToRight { tab_id: String },
    #[serde(rename = "tabs.move", rename_all = "camelCase")]
    TabsMove { tab_id: String, to_index: usize },
    #[serde(rename = "tabs.moveToNewWindow", rename_all = "camelCase")]
    TabsMoveToNewWindow { tab_id: String },
    #[serde(rename = "tabs.navigate", rename_all = "camelCase")]
    TabsNavigate { tab_id: String, input: String },
    #[serde(rename = "tabs.reload", rename_all = "camelCase")]
    TabsReload { tab_id: String },
    #[serde(rename = "tabs.stop", rename_all = "camelCase")]
    TabsStop { tab_id: String },
    #[serde(rename = "tabs.goBack", rename_all = "camelCase")]
    TabsGoBack { tab_id: String },
    #[serde(rename = "tabs.goForward", rename_all = "camelCase")]
    TabsGoForward { tab_id: String },
    #[serde(rename = "tabs.home")]
    TabsHome,
    #[serde(rename = "windows.list")]
    WindowsList,
    #[serde(rename = "windows.create")]
    WindowsCreate,
    #[serde(rename = "windows.createPrivate")]
    WindowsCreatePrivate,
    #[serde(rename = "windows.close", rename_all = "camelCase")]
    WindowsClose { window_id: Option<String> },
    #[serde(rename = "windows.reopenClosed")]
    WindowsReopenClosed,
    #[serde(rename = "settings.get", rename_all = "camelCase")]
    SettingsGet { key: String },
    #[serde(rename = "settings.set", rename_all = "camelCase")]
    SettingsSet { key: String, value: String },
    #[serde(rename = "settings.reset", rename_all = "camelCase")]
    SettingsReset { key: String },
    #[serde(rename = "bookmarks.list")]
    BookmarksList,
    #[serde(rename = "bookmarks.save", rename_all = "camelCase")]
    BookmarksSave {
        title: String,
        url: String,
        favicon_url: Option<String>,
    },
    #[serde(rename = "bookmarks.remove", rename_all = "camelCase")]
    BookmarksRemove { url: String },
    #[serde(rename = "history.list")]
    HistoryList,
    #[serde(rename = "history.remove", rename_all = "camelCase")]
    HistoryRemove { url: String },
    #[serde(rename = "history.clearRange", rename_all = "camelCase")]
    HistoryClearRange { range: String },
    #[serde(rename = "logs.list", rename_all = "camelCase")]
    LogsList { limit: usize },
    #[serde(rename = "logs.add", rename_all = "camelCase")]
    LogsAdd { level: String, message: String },
    #[serde(rename = "logs.clear")]
    LogsClear,
    #[serde(rename = "downloads.list")]
    DownloadsList,
    #[serde(rename = "downloads.remove", rename_all = "camelCase")]
    DownloadsRemove {
        url: Option<String>,
        path: Option<String>,
    },
    #[serde(rename = "downloads.open", rename_all = "camelCase")]
    DownloadsOpen { path: String },
    #[serde(rename = "downloads.reveal", rename_all = "camelCase")]
    DownloadsReveal { path: String },
    #[serde(rename = "data.clear", rename_all = "camelCase")]
    DataClear { target: Option<String> },
    #[serde(rename = "permissions.set", rename_all = "camelCase")]
    PermissionsSet {
        origin: String,
        permission: String,
        value: String,
    },
    #[serde(rename = "commands.list")]
    CommandsList,
    #[serde(rename = "commands.execute", rename_all = "camelCase")]
    CommandsExecute {
        id: String,
        args: Option<serde_json::Value>,
    },
    #[serde(rename = "ui.setSidebarWidth", rename_all = "camelCase")]
    UiSetSidebarWidth { width: f64 },
    #[serde(rename = "ui.setOverlayActive", rename_all = "camelCase")]
    UiSetOverlayActive {
        active: bool,
        width: Option<f64>,
        height: Option<f64>,
    },
}

impl ProtocolRequest {
    pub fn new(request: Request) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            id: None,
            request,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unit_request_with_empty_params() {
        let request: ProtocolRequest =
            serde_json::from_str(r#"{"version":0,"method":"app.snapshot","params":{}}"#).unwrap();

        assert_eq!(request.version, 0);
        assert_eq!(request.request, Request::AppSnapshot);
    }

    #[test]
    fn parses_bookmark_save_request() {
        let request: ProtocolRequest = serde_json::from_str(
            r#"{"version":0,"method":"bookmarks.save","params":{"title":"Example","url":"https://example.com","faviconUrl":""}}"#,
        )
        .unwrap();

        assert_eq!(
            request.request,
            Request::BookmarksSave {
                title: "Example".into(),
                url: "https://example.com".into(),
                favicon_url: Some(String::new()),
            }
        );
    }
}
