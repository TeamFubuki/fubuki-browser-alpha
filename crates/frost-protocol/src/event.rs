use serde::{Deserialize, Serialize};

use crate::state::{TabState, WindowState};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub version: u16,
    #[serde(flatten)]
    pub event: Event,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum Event {
    #[serde(rename = "tab.created")]
    TabCreated(TabState),
    #[serde(rename = "tab.updated")]
    TabUpdated(TabPatch),
    #[serde(rename = "tab.closed")]
    TabClosed(TabClosed),
    #[serde(rename = "tab.activated")]
    TabActivated(TabActivated),
    #[serde(rename = "window.created")]
    WindowCreated(WindowState),
    #[serde(rename = "window.closed")]
    WindowClosed { window_id: String },
    #[serde(rename = "setting.changed")]
    SettingChanged(SettingChanged),
    #[serde(rename = "bookmark.changed")]
    BookmarkChanged { url: String },
    #[serde(rename = "history.changed")]
    HistoryChanged { url: Option<String> },
    #[serde(rename = "download.changed")]
    DownloadChanged {
        url: Option<String>,
        path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        state: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        percent: Option<i64>,
    },
    #[serde(rename = "permission.changed")]
    PermissionChanged { origin: String, permission: String },
    #[serde(rename = "host.synced")]
    HostSynced,
    /// The one and only terminal record for a host operation.  In particular,
    /// a timeout is terminal: a late host result is rejected and cannot emit a
    /// second completion event for the same operation ID.
    #[serde(rename = "host.operationCompleted")]
    HostOperationCompleted(OperationCompleted),
    #[serde(rename = "external.audit")]
    ExternalAudit {
        command_id: String,
        capability: crate::ExternalCapability,
        allowed: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reason: Option<String>,
    },
    #[serde(rename = "external.rateLimited")]
    ExternalRateLimited {
        command_id: String,
        retry_after_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationCompleted {
    pub operation_id: String,
    pub status: OperationCompletionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationCompletionStatus {
    Succeeded,
    Failed,
    TimedOut,
}

impl OperationCompleted {
    pub fn succeeded(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            status: OperationCompletionStatus::Succeeded,
            error: None,
        }
    }

    pub fn failed(operation_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            status: OperationCompletionStatus::Failed,
            error: Some(error.into()),
        }
    }

    pub fn timed_out(operation_id: impl Into<String>) -> Self {
        Self {
            operation_id: operation_id.into(),
            status: OperationCompletionStatus::TimedOut,
            error: Some("host operation timed out".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabPatch {
    pub tab_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom_level: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_loading: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_go_back: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub can_go_forward: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabClosed {
    pub tab_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TabActivated {
    pub tab_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingChanged {
    pub key: String,
    pub value: String,
}

impl EventEnvelope {
    pub fn new(event: Event) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            event,
        }
    }

    /// Wraps an external automation event into an engine event envelope so it
    /// can be delivered over the same event channel as internal events.
    pub fn from_external(external: crate::ExternalEventEnvelope) -> Self {
        let event = match external.event {
            crate::ExternalEvent::Audit {
                command_id,
                capability,
                allowed,
                reason,
            } => Event::ExternalAudit {
                command_id,
                capability,
                allowed,
                reason,
            },
            crate::ExternalEvent::RateLimited {
                command_id,
                retry_after_ms,
            } => Event::ExternalRateLimited {
                command_id,
                retry_after_ms,
            },
        };
        Self {
            version: crate::PROTOCOL_VERSION,
            event,
        }
    }
}
