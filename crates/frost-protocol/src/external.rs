use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalCapability {
    ReadState,
    TabControl,
    Navigation,
    Bookmarks,
    History,
    Downloads,
    Debug,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalCommandEnvelope {
    pub version: u16,
    /// Correlation id for matching the response back to the caller. This is NOT
    /// an identity/authorization key; capability grant and rate-limit lookups
    /// use [`ExternalCommandEnvelope::origin`] instead.
    pub id: String,
    /// Caller origin used as the capability/rate-limit key. Never derived from
    /// `id`, and never trusted from the command's declared `capability`.
    pub origin: String,
    pub capability: ExternalCapability,
    #[serde(flatten)]
    pub command: ExternalCommand,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", content = "payload")]
pub enum ExternalCommand {
    #[serde(rename = "state.read")]
    StateRead,
    #[serde(rename = "tab.create", rename_all = "camelCase")]
    TabCreate { url: Option<String>, active: bool },
    #[serde(rename = "tab.close", rename_all = "camelCase")]
    TabClose { tab_id: String },
    #[serde(rename = "navigation.open", rename_all = "camelCase")]
    NavigationOpen { tab_id: String, input: String },
    #[serde(rename = "bookmark.save", rename_all = "camelCase")]
    BookmarkSave {
        title: String,
        url: String,
        favicon_url: Option<String>,
    },
    #[serde(rename = "history.clear", rename_all = "camelCase")]
    HistoryClear { range: String },
    #[serde(rename = "download.remove", rename_all = "camelCase")]
    DownloadRemove {
        url: Option<String>,
        path: Option<String>,
    },
    #[serde(rename = "debug.openDevTools", rename_all = "camelCase")]
    DebugOpenDevTools { tab_id: Option<String> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalEventEnvelope {
    pub version: u16,
    #[serde(flatten)]
    pub event: ExternalEvent,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum ExternalEvent {
    #[serde(rename = "external.audit", rename_all = "camelCase")]
    Audit {
        command_id: String,
        capability: ExternalCapability,
        allowed: bool,
        reason: Option<String>,
    },
    #[serde(rename = "external.rateLimited", rename_all = "camelCase")]
    RateLimited {
        command_id: String,
        retry_after_ms: u64,
    },
}

impl ExternalCommandEnvelope {
    pub fn new(
        id: impl Into<String>,
        origin: impl Into<String>,
        capability: ExternalCapability,
        command: ExternalCommand,
    ) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            id: id.into(),
            origin: origin.into(),
            capability,
            command,
        }
    }
}

impl ExternalEventEnvelope {
    pub fn new(event: ExternalEvent) -> Self {
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
    fn serializes_capability_as_snake_case() {
        let envelope = ExternalCommandEnvelope::new(
            "external-1",
            "external-origin",
            ExternalCapability::ReadState,
            ExternalCommand::StateRead,
        );

        let json = serde_json::to_value(envelope).unwrap();

        assert_eq!(json["version"], 0);
        assert_eq!(json["id"], "external-1");
        assert_eq!(json["origin"], "external-origin");
        assert_eq!(json["capability"], "read_state");
        assert_eq!(json["command"], "state.read");
    }

    #[test]
    fn keeps_correlation_id_distinct_from_origin() {
        // The correlation `id` must never be used as the authorization key.
        // Here `id` differs from `origin`; the policy layer grants by origin.
        let envelope = ExternalCommandEnvelope::new(
            "correlation-abc",
            "external-origin",
            ExternalCapability::ReadState,
            ExternalCommand::StateRead,
        );

        let json = serde_json::to_value(envelope).unwrap();

        assert_eq!(json["id"], "correlation-abc");
        assert_eq!(json["origin"], "external-origin");
        assert_ne!(json["id"], json["origin"]);
    }

    #[test]
    fn round_trips_origin_via_json() {
        let envelope = ExternalCommandEnvelope::new(
            "external-1",
            "external-origin",
            ExternalCapability::ReadState,
            ExternalCommand::StateRead,
        );
        let json = serde_json::to_string(&envelope).unwrap();
        let parsed: ExternalCommandEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.origin, "external-origin");
        assert_eq!(parsed.id, "external-1");
    }
}
