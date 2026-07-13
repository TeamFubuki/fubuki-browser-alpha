use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCommandEnvelope {
    pub version: u16,
    /// Correlates a command with exactly one terminal host result.  Rust owns
    /// this identifier; hosts must return it unchanged as `operationId`.
    #[serde(alias = "id")]
    pub operation_id: String,
    #[serde(flatten)]
    pub command: HostCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", content = "payload")]
pub enum HostCommand {
    #[serde(rename = "page.create", rename_all = "camelCase")]
    PageCreate {
        tab_id: String,
        window_id: String,
        url: String,
    },
    #[serde(rename = "page.close", rename_all = "camelCase")]
    PageClose { tab_id: String },
    #[serde(rename = "page.navigate", rename_all = "camelCase")]
    PageNavigate { tab_id: String, url: String },
    #[serde(rename = "page.reload", rename_all = "camelCase")]
    PageReload { tab_id: String },
    #[serde(rename = "page.stop", rename_all = "camelCase")]
    PageStop { tab_id: String },
    #[serde(rename = "page.goBack", rename_all = "camelCase")]
    PageGoBack { tab_id: String },
    #[serde(rename = "page.goForward", rename_all = "camelCase")]
    PageGoForward { tab_id: String },
    #[serde(rename = "window.create", rename_all = "camelCase")]
    WindowCreate { window_id: String, is_private: bool },
    #[serde(rename = "window.close", rename_all = "camelCase")]
    WindowClose { window_id: String },
    /// Requests creation of a separate in-memory private runtime.  It
    /// deliberately carries no `windowId`: the persistent core must never
    /// allocate or own private-window state.
    #[serde(rename = "runtime.createPrivate")]
    RuntimeCreatePrivate,
    #[serde(rename = "file.open", rename_all = "camelCase")]
    FileOpen { path: String },
    #[serde(rename = "file.reveal", rename_all = "camelCase")]
    FileReveal { path: String },
    #[serde(rename = "devtools.open", rename_all = "camelCase")]
    DevToolsOpen { tab_id: Option<String> },
    #[serde(rename = "browsingData.clear", rename_all = "camelCase")]
    BrowsingDataClear { target: String },
    #[serde(rename = "ui.overlay.set", rename_all = "camelCase")]
    UiOverlaySet {
        active: bool,
        width: Option<f64>,
        height: Option<f64>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCommandResultEnvelope {
    pub version: u16,
    /// Compatibility alias for hosts built before operation IDs were named
    /// consistently. New hosts must emit `operationId`.
    #[serde(alias = "commandId")]
    pub operation_id: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostEventEnvelope {
    pub version: u16,
    #[serde(flatten)]
    pub event: HostEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum HostEvent {
    #[serde(rename = "page.created", rename_all = "camelCase")]
    PageCreated {
        tab_id: String,
        window_id: String,
        url: String,
    },
    #[serde(rename = "page.closed", rename_all = "camelCase")]
    PageClosed { tab_id: String },
    #[serde(rename = "page.titleChanged", rename_all = "camelCase")]
    PageTitleChanged { tab_id: String, title: String },
    #[serde(rename = "page.urlChanged", rename_all = "camelCase")]
    PageUrlChanged { tab_id: String, url: String },
    #[serde(rename = "page.faviconChanged", rename_all = "camelCase")]
    PageFaviconChanged { tab_id: String, favicon_url: String },
    #[serde(rename = "page.loadingChanged", rename_all = "camelCase")]
    PageLoadingChanged { tab_id: String, is_loading: bool },
    #[serde(rename = "page.navigationStateChanged", rename_all = "camelCase")]
    PageNavigationStateChanged {
        tab_id: String,
        can_go_back: bool,
        can_go_forward: bool,
    },
    #[serde(rename = "page.loadFailed", rename_all = "camelCase")]
    PageLoadFailed { tab_id: String, error_text: String },
    /// A CEF navigation completed. FrostEngine records history from its own
    /// current tab state, keeping persistence out of the native host.
    #[serde(rename = "page.navigationFinished", rename_all = "camelCase")]
    PageNavigationFinished { tab_id: String },
    #[serde(rename = "download.updated", rename_all = "camelCase")]
    DownloadUpdated {
        url: String,
        path: String,
        state: String,
        percent: i64,
    },
    #[serde(rename = "permission.changed", rename_all = "camelCase")]
    PermissionChanged {
        origin: String,
        permission: String,
        value: String,
    },
    #[serde(rename = "window.focused", rename_all = "camelCase")]
    WindowFocused { window_id: String },
    #[serde(rename = "window.closed", rename_all = "camelCase")]
    WindowClosed { window_id: String },
}

impl HostCommandEnvelope {
    pub fn new(operation_id: impl Into<String>, command: HostCommand) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            operation_id: operation_id.into(),
            command,
        }
    }
}

impl HostCommandResultEnvelope {
    pub fn success(operation_id: impl Into<String>) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            operation_id: operation_id.into(),
            ok: true,
            error: None,
        }
    }

    pub fn failure(operation_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            operation_id: operation_id.into(),
            ok: false,
            error: Some(error.into()),
        }
    }
}

impl HostEventEnvelope {
    pub fn new(event: HostEvent) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            event,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_host_command_envelope() {
        let envelope = HostCommandEnvelope::new(
            "cmd-1",
            HostCommand::PageNavigate {
                tab_id: "tab-1".into(),
                url: "https://example.com".into(),
            },
        );

        let json = serde_json::to_value(envelope).unwrap();

        assert_eq!(json["version"], 0);
        assert_eq!(json["operationId"], "cmd-1");
        assert_eq!(json["command"], "page.navigate");
        assert_eq!(json["payload"]["tabId"], "tab-1");
        assert_eq!(json["payload"]["url"], "https://example.com");
    }

    #[test]
    fn accepts_legacy_host_result_command_id() {
        let result: HostCommandResultEnvelope = serde_json::from_value(serde_json::json!({
            "version": 0,
            "commandId": "cmd-1",
            "ok": false,
            "error": "not implemented"
        }))
        .unwrap();
        assert_eq!(result.operation_id, "cmd-1");
    }

    #[test]
    fn private_runtime_command_has_no_persistent_window_identifier() {
        let command =
            HostCommandEnvelope::new("operation-private", HostCommand::RuntimeCreatePrivate);
        let json = serde_json::to_value(command).unwrap();

        assert_eq!(json["command"], "runtime.createPrivate");
        assert!(json["payload"].is_null());
        assert!(json.get("windowId").is_none());
    }
}
