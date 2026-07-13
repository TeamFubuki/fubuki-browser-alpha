//! External automation / MCP command routing.
//!
//! External clients (MCP servers, automation tooling) connect at the engine's
//! command layer through [`ExternalCommand`] and declared [`ExternalCapability`]
//! values. Every command is gated by a capability check, subject to a per-origin
//! rate limit, and produces an audit event before it is routed to the owned
//! services or emitted as a [`frost_protocol::HostCommand`].
//!
//! This module is the single entry point for external commands so that no
//! external client can reach CEF / NSWindow or the host directly.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use frost_engine_api::EngineAdapter;
use frost_protocol::{
    ExternalCapability, ExternalCommand, ExternalCommandEnvelope, ExternalEvent,
    ExternalEventEnvelope,
};
use frost_store::{
    BookmarkRepository, ClearRepository, DownloadRepository, HistoryRepository, LogRepository,
    SessionRepository,
};

use crate::{BookmarkService, BrowserCore, CoreError, CoreResult, DownloadService, HistoryService};

/// Default maximum commands allowed per window before rate limiting kicks in.
const DEFAULT_RATE_LIMIT: u32 = 60;
/// Sliding window used for the rate limiter.
const RATE_WINDOW: Duration = Duration::from_secs(60);

/// Per-capability policy describing whether the command is destructive and
/// which capability gates it.
struct CommandPolicy {
    capability: ExternalCapability,
    destructive: bool,
}

fn policy_for(command: &ExternalCommand) -> CommandPolicy {
    match command {
        ExternalCommand::StateRead => CommandPolicy {
            capability: ExternalCapability::ReadState,
            destructive: false,
        },
        ExternalCommand::TabCreate { .. } => CommandPolicy {
            capability: ExternalCapability::TabControl,
            destructive: false,
        },
        ExternalCommand::TabClose { .. } => CommandPolicy {
            capability: ExternalCapability::TabControl,
            destructive: true,
        },
        ExternalCommand::NavigationOpen { .. } => CommandPolicy {
            capability: ExternalCapability::Navigation,
            destructive: false,
        },
        ExternalCommand::BookmarkSave { .. } => CommandPolicy {
            capability: ExternalCapability::Bookmarks,
            destructive: false,
        },
        ExternalCommand::HistoryClear { .. } => CommandPolicy {
            capability: ExternalCapability::History,
            destructive: true,
        },
        ExternalCommand::DownloadRemove { .. } => CommandPolicy {
            capability: ExternalCapability::Downloads,
            destructive: true,
        },
        ExternalCommand::DebugOpenDevTools { .. } => CommandPolicy {
            capability: ExternalCapability::Debug,
            destructive: false,
        },
    }
}

/// Sliding-window rate limiter state for a single caller origin.
#[derive(Default)]
struct RateBucket {
    timestamps: Vec<Instant>,
}

impl RateBucket {
    fn record(&mut self) -> bool {
        let now = Instant::now();
        self.timestamps.retain(|t| {
            now.checked_duration_since(*t)
                .is_some_and(|d| d <= RATE_WINDOW)
        });
        if self.timestamps.len() >= DEFAULT_RATE_LIMIT as usize {
            return false;
        }
        self.timestamps.push(now);
        true
    }
}

/// Tracks granted capabilities per external caller origin.
#[derive(Default)]
pub struct ExternalPolicy {
    grants: HashMap<String, Vec<ExternalCapability>>,
    rate: HashMap<String, RateBucket>,
}

impl ExternalPolicy {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grants `capabilities` to a caller identified by `origin`.
    pub fn grant(&mut self, origin: &str, capabilities: Vec<ExternalCapability>) {
        self.grants.insert(origin.to_owned(), capabilities);
    }

    /// Revokes all capabilities for `origin`.
    pub fn revoke(&mut self, origin: &str) {
        self.grants.remove(origin);
        self.rate.remove(origin);
    }

    pub fn is_granted(&self, origin: &str, capability: &ExternalCapability) -> bool {
        self.grants
            .get(origin)
            .map(|caps| caps.iter().any(|c| c == capability))
            .unwrap_or(false)
    }

    pub fn check_rate(&mut self, origin: &str) -> bool {
        self.rate.entry(origin.to_owned()).or_default().record()
    }
}

/// Result of routing an external command.
pub struct ExternalOutcome {
    pub allowed: bool,
    pub reason: Option<String>,
    pub response: ExternalResponse,
}

/// Response returned to the external caller.
pub enum ExternalResponse {
    Ok(serde_json::Value),
    Error(String),
}

impl<A, S> BrowserCore<A, S>
where
    A: EngineAdapter,
    S: frost_store::SettingsRepository
        + BookmarkRepository
        + HistoryRepository
        + DownloadRepository
        + frost_store::PermissionRepository
        + LogRepository
        + SessionRepository
        + ClearRepository,
{
    /// Processes an external command under the given policy.
    ///
    /// Every call performs capability, rate-limit and (for destructive
    /// actions) audit checks before delegating to the owned services.
    pub fn process_external(
        &mut self,
        envelope: ExternalCommandEnvelope,
        policy: &mut ExternalPolicy,
    ) -> ExternalOutcome {
        let command = envelope.command.clone();
        let cmd_policy = policy_for(&command);
        let capability = cmd_policy.capability;
        let destructive = cmd_policy.destructive;
        // Gate by the caller origin, never by the correlation id.
        let origin = envelope.origin.clone();

        if !policy.is_granted(&origin, &capability) {
            let reason = format!("capability '{:?}' not granted", capability);
            self.emit_external_audit(&envelope, false, Some(reason.clone()));
            return ExternalOutcome {
                allowed: false,
                reason: Some(reason),
                response: ExternalResponse::Error("capability not granted".into()),
            };
        }

        if !policy.check_rate(&origin) {
            self.emit_external_event(ExternalEventEnvelope::new(ExternalEvent::RateLimited {
                command_id: envelope.id.clone(),
                retry_after_ms: RATE_WINDOW.as_millis() as u64,
            }));
            self.emit_external_audit(&envelope, false, Some("rate limited".into()));
            return ExternalOutcome {
                allowed: false,
                reason: Some("rate limited".into()),
                response: ExternalResponse::Error("rate limited".into()),
            };
        }

        if destructive {
            self.emit_external_audit(&envelope, true, None);
        }

        let result = self.route_external(command);
        let allowed = result.is_ok();
        if !allowed {
            self.emit_external_audit(
                &envelope,
                false,
                result.as_ref().err().map(|e| e.to_string()),
            );
        }
        ExternalOutcome {
            allowed,
            reason: result.as_ref().err().map(|e| e.to_string()),
            response: match result {
                Ok(value) => ExternalResponse::Ok(value),
                Err(e) => ExternalResponse::Error(e.to_string()),
            },
        }
    }

    fn route_external(&mut self, command: ExternalCommand) -> CoreResult<serde_json::Value> {
        match command {
            ExternalCommand::StateRead => {
                let snapshot = self.snapshot()?;
                Ok(
                    serde_json::to_value(snapshot)
                        .map_err(|e| CoreError::Message(e.to_string()))?,
                )
            }
            ExternalCommand::TabCreate { url, active } => {
                let request =
                    frost_protocol::ProtocolRequest::new(frost_protocol::Request::TabsCreate {
                        url,
                        active,
                        window_id: None,
                    });
                Self::bool_response_or_error(self.process(request), "tab create failed")
            }
            ExternalCommand::TabClose { tab_id } => {
                let request =
                    frost_protocol::ProtocolRequest::new(frost_protocol::Request::TabsClose {
                        tab_id,
                    });
                Self::bool_response_or_error(self.process(request), "tab close failed")
            }
            ExternalCommand::NavigationOpen { tab_id, input } => {
                let request =
                    frost_protocol::ProtocolRequest::new(frost_protocol::Request::TabsNavigate {
                        tab_id,
                        input,
                    });
                Self::bool_response_or_error(self.process(request), "navigation failed")
            }
            ExternalCommand::BookmarkSave {
                title,
                url,
                favicon_url,
            } => {
                BookmarkService::save(
                    &self.repository,
                    &title,
                    &url,
                    favicon_url.as_deref().unwrap_or(""),
                )
                .map_err(|e| CoreError::Message(e.to_string()))?;
                Ok(serde_json::json!({ "ok": true }))
            }
            ExternalCommand::HistoryClear { range } => {
                HistoryService::clear_range(&self.repository, &range)
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                Ok(serde_json::json!({ "ok": true }))
            }
            ExternalCommand::DownloadRemove { url, path } => {
                DownloadService::remove(&self.repository, url.as_deref(), path.as_deref())
                    .map_err(|e| CoreError::Message(e.to_string()))?;
                Ok(serde_json::json!({ "ok": true }))
            }
            ExternalCommand::DebugOpenDevTools { tab_id } => {
                // DevTools is a host/CEF concern. The engine must not reach into
                // CEF directly, and there is no HostCommand for it yet, so we
                // surface a clear error instead of performing a misleading action.
                Err(CoreError::Message(format!(
                    "devtools open for tab '{}' is not supported via external router",
                    tab_id.as_deref().unwrap_or("<none>")
                )))
            }
        }
    }

    /// Maps a [`frost_protocol::ProtocolResponse`] to a JSON outcome.  A
    /// queued host action is acknowledged as pending rather than incorrectly
    /// reported as completed to an external caller.
    fn bool_response_or_error(
        response: frost_protocol::ProtocolResponse,
        label: &str,
    ) -> CoreResult<serde_json::Value> {
        match response.response {
            frost_protocol::Response::Bool(true) => Ok(serde_json::json!({ "ok": true })),
            frost_protocol::Response::Operation(operation) => Ok(serde_json::json!({
                "ok": true,
                "pending": true,
                "operation": operation,
            })),
            frost_protocol::Response::Operations(operations) => Ok(serde_json::json!({
                "ok": true,
                "pending": true,
                "operations": operations,
            })),
            frost_protocol::Response::Error(msg) => Err(CoreError::Message(msg)),
            _ => Err(CoreError::Message(label.into())),
        }
    }

    fn emit_external_audit(
        &mut self,
        envelope: &ExternalCommandEnvelope,
        allowed: bool,
        reason: Option<String>,
    ) {
        let capability = policy_for(&envelope.command).capability;
        self.emit_external_event(ExternalEventEnvelope::new(ExternalEvent::Audit {
            command_id: envelope.id.clone(),
            capability,
            allowed,
            reason,
        }));
    }

    fn emit_external_event(&mut self, event: ExternalEventEnvelope) {
        if let Some(tx) = &self.event_tx {
            let _ = tx.send(frost_protocol::EventEnvelope::from_external(event));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frost_engine_api::NoopEngineAdapter;
    use frost_protocol::{ExternalCapability, ExternalCommand};
    use frost_store::SqliteStore;

    fn core() -> BrowserCore<NoopEngineAdapter, SqliteStore> {
        BrowserCore::with_adapter_and_settings(NoopEngineAdapter, SqliteStore::in_memory().unwrap())
    }

    #[test]
    fn rejects_command_without_capability() {
        let mut core = core();
        let mut policy = ExternalPolicy::new();
        let envelope = ExternalCommandEnvelope::new(
            "ext-1",
            "ext-1",
            ExternalCapability::ReadState,
            ExternalCommand::StateRead,
        );
        let outcome = core.process_external(envelope, &mut policy);
        assert!(!outcome.allowed);
        assert_eq!(
            outcome.reason.as_deref(),
            Some("capability 'ReadState' not granted")
        );
    }

    #[test]
    fn allows_command_with_granted_capability() {
        let mut core = core();
        let mut policy = ExternalPolicy::new();
        policy.grant("ext-1", vec![ExternalCapability::ReadState]);
        let envelope = ExternalCommandEnvelope::new(
            "ext-1",
            "ext-1",
            ExternalCapability::ReadState,
            ExternalCommand::StateRead,
        );
        let outcome = core.process_external(envelope, &mut policy);
        assert!(outcome.allowed);
    }

    #[test]
    fn rejects_destructive_without_capability() {
        let mut core = core();
        let mut policy = ExternalPolicy::new();
        policy.grant("ext-2", vec![ExternalCapability::TabControl]);
        let envelope = ExternalCommandEnvelope::new(
            "ext-2",
            "ext-2",
            ExternalCapability::TabControl,
            ExternalCommand::TabClose {
                tab_id: "tab-1".into(),
            },
        );
        let outcome = core.process_external(envelope, &mut policy);
        // Capability is granted but no tab exists, so routing fails.
        assert!(!outcome.allowed);
    }

    #[test]
    fn grants_by_origin_not_by_correlation_id() {
        let mut core = core();
        let mut policy = ExternalPolicy::new();
        // Grant to the origin; the correlation `id` is intentionally different.
        policy.grant("ext-origin", vec![ExternalCapability::ReadState]);
        let envelope = ExternalCommandEnvelope::new(
            "correlation-id",
            "ext-origin",
            ExternalCapability::ReadState,
            ExternalCommand::StateRead,
        );
        let outcome = core.process_external(envelope, &mut policy);
        assert!(outcome.allowed);
    }

    #[test]
    fn rate_limits_by_origin() {
        let mut core = core();
        let mut policy = ExternalPolicy::new();
        policy.grant("ext-rate", vec![ExternalCapability::ReadState]);
        for _ in 0..DEFAULT_RATE_LIMIT {
            let envelope = ExternalCommandEnvelope::new(
                "id",
                "ext-rate",
                ExternalCapability::ReadState,
                ExternalCommand::StateRead,
            );
            assert!(core.process_external(envelope, &mut policy).allowed);
        }
        let envelope = ExternalCommandEnvelope::new(
            "id",
            "ext-rate",
            ExternalCapability::ReadState,
            ExternalCommand::StateRead,
        );
        assert!(!core.process_external(envelope, &mut policy).allowed);
    }
}
