use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use frost_engine_api::{EngineAdapter, HostCommandId};
use frost_protocol::{TabState, WindowState};

use crate::{TabService, WindowService};

const HOST_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

/// State captured before a host-facing mutation so the core can restore it.
#[derive(Debug, Clone)]
pub(crate) struct StateSnapshot {
    tabs: Vec<TabState>,
    windows: Vec<WindowState>,
    active_window_id: Option<String>,
    closed_tabs: Vec<TabState>,
    closed_windows: Vec<ClosedWindow>,
}

impl StateSnapshot {
    pub(crate) fn capture(
        tabs: &TabService,
        windows: &WindowService,
        closed_tabs: &[TabState],
        closed_windows: &[ClosedWindow],
    ) -> Self {
        Self {
            tabs: tabs.list(),
            windows: windows.list(),
            active_window_id: windows.active_window_id().map(ToOwned::to_owned),
            closed_tabs: closed_tabs.to_vec(),
            closed_windows: closed_windows.to_vec(),
        }
    }

    pub(crate) fn restore(
        &self,
        tabs: &mut TabService,
        windows: &mut WindowService,
        closed_tabs: &mut Vec<TabState>,
        closed_windows: &mut Vec<ClosedWindow>,
    ) {
        tabs.replace_all(self.tabs.clone());
        windows.replace_all(self.windows.clone(), self.active_window_id.clone());
        *closed_tabs = self.closed_tabs.clone();
        *closed_windows = self.closed_windows.clone();
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ClosedWindow {
    pub(crate) window: WindowState,
    pub(crate) tabs: Vec<TabState>,
}

/// A mutation whose local state can be undone if its host command fails.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) enum PendingOperation {
    StateSnapshot(StateSnapshot),
    TabCreated {
        tab_id: String,
        window_id: String,
    },
    TabMoved {
        tab_id: String,
        from_window_id: String,
        to_window_id: String,
        from_index: usize,
        to_index: usize,
    },
    TabActivated {
        tab_id: String,
        window_id: String,
        previous_active_tab_id: Option<String>,
    },
    WindowCreated {
        window_id: String,
    },
    TabClosed {
        tab_id: String,
        window_id: String,
        was_active: bool,
        tab_state: TabState,
        previous_active_tab_id: Option<String>,
        replacement_tab: Option<TabState>,
    },
    TabsCloseOther {
        keep_tab_id: String,
        closed_tabs: Vec<TabState>,
        window_id: String,
        was_active: bool,
        previous_active_tab_id: Option<String>,
    },
    TabsCloseToRight {
        anchor_tab_id: String,
        closed_tabs: Vec<TabState>,
        window_id: String,
        was_active: bool,
        previous_active_tab_id: Option<String>,
    },
    TabMoveToNewWindow {
        tab_id: String,
        original_window_id: String,
        new_window_id: String,
        new_window_is_private: bool,
        empty_tab_created: Option<TabState>,
    },
    TabPin {
        tab_id: String,
        previous_pinned: bool,
    },
    TabNavigate {
        tab_id: String,
        previous_url: String,
        previous_error_text: String,
        previous_is_loading: bool,
    },
}

#[derive(Debug, Clone)]
pub(crate) enum PendingEntry {
    Single(PendingOperation),
    Transaction {
        group_id: String,
        operation: PendingOperation,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionResult {
    Completed,
    UnknownCommand,
}

#[derive(Debug, Clone)]
pub(crate) enum FailureResult {
    Rollback(PendingOperation),
    UnknownCommand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RollbackResult {
    Applied,
}

/// Owns all in-flight host mutations and their transaction-group lifecycle.
#[derive(Debug, Default)]
pub(crate) struct PendingOperations {
    entries: HashMap<String, PendingEntry>,
    pending_since: HashMap<String, SystemTime>,
    next_local_id: u64,
}

impl PendingOperations {
    pub(crate) fn record(
        &mut self,
        command_id: HostCommandId,
        operation: PendingOperation,
    ) -> HostCommandId {
        let id = self.allocate_id(command_id);
        self.entries
            .insert(id.clone(), PendingEntry::Single(operation));
        self.pending_since.insert(id.clone(), SystemTime::now());
        id
    }

    pub(crate) fn record_transaction(
        &mut self,
        command_ids: Vec<HostCommandId>,
        operation: PendingOperation,
    ) -> Vec<HostCommandId> {
        let group_id = format!("pending-group-{}", uuid::Uuid::new_v4());
        let ids = if command_ids.is_empty() {
            vec![String::new()]
        } else {
            command_ids
        };
        ids.into_iter()
            .map(|command_id| {
                let id = self.allocate_id(command_id);
                self.entries.insert(
                    id.clone(),
                    PendingEntry::Transaction {
                        group_id: group_id.clone(),
                        operation: operation.clone(),
                    },
                );
                self.pending_since.insert(id.clone(), SystemTime::now());
                id
            })
            .collect()
    }

    pub(crate) fn complete(&mut self, command_id: &str) -> CompletionResult {
        if self.entries.remove(command_id).is_some() {
            self.pending_since.remove(command_id);
            CompletionResult::Completed
        } else {
            CompletionResult::UnknownCommand
        }
    }

    /// Removes a failed command. For a transaction, all sibling commands are
    /// removed with it and the operation is returned exactly once.
    pub(crate) fn fail(&mut self, command_id: &str) -> FailureResult {
        let Some(entry) = self.entries.remove(command_id) else {
            return FailureResult::UnknownCommand;
        };
        self.pending_since.remove(command_id);

        match entry {
            PendingEntry::Single(operation) => FailureResult::Rollback(operation),
            PendingEntry::Transaction {
                group_id,
                operation,
            } => {
                let sibling_ids: Vec<String> = self
                    .entries
                    .iter()
                    .filter_map(|(id, entry)| match entry {
                        PendingEntry::Transaction {
                            group_id: entry_group_id,
                            ..
                        } if entry_group_id == &group_id => Some(id.clone()),
                        _ => None,
                    })
                    .collect();
                for sibling_id in sibling_ids {
                    self.entries.remove(&sibling_id);
                    self.pending_since.remove(&sibling_id);
                }
                FailureResult::Rollback(operation)
            }
        }
    }

    pub(crate) fn expire(&mut self, now: SystemTime) -> Vec<HostCommandId> {
        let expired: Vec<String> = self
            .pending_since
            .iter()
            .filter(|(_, started)| {
                now.duration_since(**started).unwrap_or_default() >= HOST_COMMAND_TIMEOUT
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in &expired {
            self.pending_since.remove(id);
            self.entries.remove(id);
        }
        expired
    }

    fn allocate_id(&mut self, command_id: HostCommandId) -> HostCommandId {
        if !command_id.is_empty() {
            return command_id;
        }
        loop {
            let id = format!("local-pending-{}", self.next_local_id);
            self.next_local_id += 1;
            if !self.entries.contains_key(&id) {
                return id;
            }
        }
    }
}

/// Mutable state needed to apply a rollback without exposing BrowserCore's
/// pending maps or rollback branches to the coordinator.
pub(crate) struct RollbackContext<'a, A: EngineAdapter> {
    pub(crate) adapter: &'a mut A,
    pub(crate) tabs: &'a mut TabService,
    pub(crate) windows: &'a mut WindowService,
    pub(crate) closed_tabs: &'a mut Vec<TabState>,
    pub(crate) closed_windows: &'a mut Vec<ClosedWindow>,
}

impl<A: EngineAdapter> RollbackContext<'_, A> {
    pub(crate) fn apply(&mut self, operation: PendingOperation) -> RollbackResult {
        match operation {
            PendingOperation::StateSnapshot(snapshot) => snapshot.restore(
                self.tabs,
                self.windows,
                self.closed_tabs,
                self.closed_windows,
            ),
            PendingOperation::TabCreated { tab_id, window_id } => {
                self.windows.detach_tab(&tab_id);
                self.tabs.remove_tab(&tab_id);
                if self.tabs.tabs_in_window(&window_id).is_empty() {
                    self.windows.close_window(&window_id);
                }
            }
            PendingOperation::TabMoved {
                tab_id,
                from_window_id,
                to_window_id,
                from_index,
                to_index: _,
            } => {
                self.tabs.move_tab_to_window(&tab_id, &from_window_id);
                self.windows.move_tab_to_window(&tab_id, &from_window_id);
                self.tabs.move_tab(&tab_id, from_index);
                self.windows.move_tab_in_window(&tab_id, from_index);
                if self.tabs.tabs_in_window(&to_window_id).is_empty() {
                    self.windows.close_window(&to_window_id);
                }
            }
            PendingOperation::TabActivated {
                tab_id: _,
                window_id: _,
                previous_active_tab_id,
            } => {
                if let Some(previous_id) = previous_active_tab_id {
                    self.tabs.activate_tab(&previous_id);
                    self.windows.set_active_tab(&previous_id);
                }
            }
            PendingOperation::WindowCreated { window_id } => {
                self.windows.close_window(&window_id);
            }
            PendingOperation::TabClosed {
                tab_id,
                window_id,
                was_active,
                tab_state,
                previous_active_tab_id,
                replacement_tab,
            } => {
                if let Some(replacement_tab) = replacement_tab {
                    self.windows.detach_tab(&replacement_tab.id);
                    self.tabs.remove_tab(&replacement_tab.id);
                }
                self.closed_tabs.retain(|closed| closed.id != tab_id);
                self.tabs.upsert_tab(tab_state);
                self.windows.attach_tab(&window_id, &tab_id, was_active);
                if let Some(previous_id) = previous_active_tab_id {
                    self.tabs.activate_tab(&previous_id);
                    self.windows.set_active_tab(&previous_id);
                } else if was_active {
                    self.tabs.activate_tab(&tab_id);
                    self.windows.set_active_tab(&tab_id);
                }
            }
            PendingOperation::TabsCloseOther {
                keep_tab_id,
                closed_tabs,
                window_id,
                was_active,
                previous_active_tab_id,
            } => {
                restore_closed_tabs(
                    self.tabs,
                    self.windows,
                    self.closed_tabs,
                    &window_id,
                    closed_tabs,
                );
                restore_active_tab(
                    self.tabs,
                    self.windows,
                    previous_active_tab_id,
                    was_active.then_some(keep_tab_id),
                );
            }
            PendingOperation::TabsCloseToRight {
                anchor_tab_id,
                closed_tabs,
                window_id,
                was_active,
                previous_active_tab_id,
            } => {
                restore_closed_tabs(
                    self.tabs,
                    self.windows,
                    self.closed_tabs,
                    &window_id,
                    closed_tabs,
                );
                restore_active_tab(
                    self.tabs,
                    self.windows,
                    previous_active_tab_id,
                    was_active.then_some(anchor_tab_id),
                );
            }
            PendingOperation::TabMoveToNewWindow {
                tab_id,
                original_window_id,
                new_window_id,
                new_window_is_private: _,
                empty_tab_created,
            } => {
                self.tabs.move_tab_to_window(&tab_id, &original_window_id);
                self.windows
                    .move_tab_to_window(&tab_id, &original_window_id);
                self.windows.close_window(&new_window_id);
                if let Some(empty_tab) = empty_tab_created {
                    self.windows.detach_tab(&empty_tab.id);
                    self.tabs.remove_tab(&empty_tab.id);
                }
            }
            PendingOperation::TabPin {
                tab_id,
                previous_pinned,
            } => {
                self.tabs.pin_tab(&tab_id, previous_pinned);
                let _ = self.adapter.pin_page(&tab_id, previous_pinned);
            }
            PendingOperation::TabNavigate {
                tab_id,
                previous_url,
                previous_error_text,
                previous_is_loading,
            } => {
                if let Some(mut tab) = self.tabs.get_tab(&tab_id) {
                    tab.url = previous_url;
                    tab.error_text = previous_error_text;
                    tab.is_loading = previous_is_loading;
                    self.tabs.upsert_tab(tab);
                }
            }
        }
        RollbackResult::Applied
    }
}

fn restore_closed_tabs(
    tabs: &mut TabService,
    windows: &mut WindowService,
    closed_tabs: &mut Vec<TabState>,
    window_id: &str,
    restored_tabs: Vec<TabState>,
) {
    for tab in restored_tabs {
        closed_tabs.retain(|closed| closed.id != tab.id);
        tabs.upsert_tab(tab.clone());
        windows.attach_tab(window_id, &tab.id, tab.is_active);
    }
}

fn restore_active_tab(
    tabs: &mut TabService,
    windows: &mut WindowService,
    previous_active_tab_id: Option<String>,
    fallback_tab_id: Option<String>,
) {
    if let Some(previous_id) = previous_active_tab_id {
        tabs.activate_tab(&previous_id);
        windows.set_active_tab(&previous_id);
    } else if let Some(fallback_id) = fallback_tab_id {
        tabs.activate_tab(&fallback_id);
        windows.set_active_tab(&fallback_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frost_engine_api::NoopEngineAdapter;

    fn operation() -> PendingOperation {
        PendingOperation::WindowCreated {
            window_id: "window-1".into(),
        }
    }

    #[test]
    fn record_returns_command_id_for_host_command() {
        let mut pending = PendingOperations::default();
        assert_eq!(pending.record("cmd-1".into(), operation()), "cmd-1");
    }

    #[test]
    fn record_allocates_unique_id_for_synchronous_command() {
        let mut pending = PendingOperations::default();
        let first = pending.record(String::new(), operation());
        let second = pending.record(String::new(), operation());
        assert_ne!(first, second);
    }

    #[test]
    fn complete_removes_successful_operation() {
        let mut pending = PendingOperations::default();
        let id = pending.record("cmd-success".into(), operation());
        assert_eq!(pending.complete(&id), CompletionResult::Completed);
        assert!(matches!(pending.fail(&id), FailureResult::UnknownCommand));
    }

    #[test]
    fn complete_unknown_command_is_safe() {
        let mut pending = PendingOperations::default();
        assert_eq!(
            pending.complete("unknown"),
            CompletionResult::UnknownCommand
        );
    }

    #[test]
    fn failed_single_operation_requests_rollback() {
        let mut pending = PendingOperations::default();
        let id = pending.record("cmd-failure".into(), operation());
        assert!(matches!(pending.fail(&id), FailureResult::Rollback(_)));
    }

    #[test]
    fn failed_unknown_command_does_not_request_rollback() {
        let mut pending = PendingOperations::default();
        assert!(matches!(
            pending.fail("unknown"),
            FailureResult::UnknownCommand
        ));
    }

    #[test]
    fn failed_transaction_removes_all_siblings() {
        let mut pending = PendingOperations::default();
        let ids = pending.record_transaction(
            vec!["cmd-a".into(), "cmd-b".into(), "cmd-c".into()],
            operation(),
        );
        assert!(matches!(pending.fail(&ids[1]), FailureResult::Rollback(_)));
        assert_eq!(pending.complete(&ids[0]), CompletionResult::UnknownCommand);
        assert!(matches!(
            pending.fail(&ids[2]),
            FailureResult::UnknownCommand
        ));
    }

    #[test]
    fn transaction_rolls_back_only_once_after_duplicate_failure() {
        let mut pending = PendingOperations::default();
        let ids = pending.record_transaction(vec!["cmd-a".into(), "cmd-b".into()], operation());
        assert!(matches!(pending.fail(&ids[0]), FailureResult::Rollback(_)));
        assert!(matches!(
            pending.fail(&ids[0]),
            FailureResult::UnknownCommand
        ));
        assert!(matches!(
            pending.fail(&ids[1]),
            FailureResult::UnknownCommand
        ));
    }

    #[test]
    fn completing_one_transaction_member_keeps_other_members_pending() {
        let mut pending = PendingOperations::default();
        let ids = pending.record_transaction(vec!["cmd-a".into(), "cmd-b".into()], operation());
        assert_eq!(pending.complete(&ids[0]), CompletionResult::Completed);
        assert!(matches!(pending.fail(&ids[1]), FailureResult::Rollback(_)));
    }

    #[test]
    fn expiration_removes_stale_operation_without_rollback() {
        let mut pending = PendingOperations::default();
        let id = pending.record("cmd-timeout".into(), operation());
        let expired = pending.expire(SystemTime::now() + HOST_COMMAND_TIMEOUT);
        assert_eq!(expired, vec![id.clone()]);
        assert!(matches!(pending.fail(&id), FailureResult::UnknownCommand));
    }

    #[test]
    fn expiration_keeps_fresh_operation() {
        let mut pending = PendingOperations::default();
        let id = pending.record("cmd-fresh".into(), operation());
        assert!(pending.expire(SystemTime::now()).is_empty());
        assert!(matches!(pending.fail(&id), FailureResult::Rollback(_)));
    }

    #[test]
    fn state_snapshot_rollback_restores_core_state() {
        let mut windows = WindowService::new();
        let window_id = windows.create_window(false);
        let mut tabs = TabService::new(window_id.clone());
        let original = tabs.create_tab(window_id.clone(), "https://before.example".into(), true);
        windows.attach_tab(&window_id, &original.id, true);
        let snapshot = StateSnapshot::capture(&tabs, &windows, &[], &[]);
        let changed = tabs.create_tab(window_id.clone(), "https://after.example".into(), true);
        windows.attach_tab(&window_id, &changed.id, true);

        let mut adapter = NoopEngineAdapter;
        let mut closed_tabs = Vec::new();
        let mut closed_windows = Vec::new();
        let result = RollbackContext {
            adapter: &mut adapter,
            tabs: &mut tabs,
            windows: &mut windows,
            closed_tabs: &mut closed_tabs,
            closed_windows: &mut closed_windows,
        }
        .apply(PendingOperation::StateSnapshot(snapshot));

        assert_eq!(result, RollbackResult::Applied);
        assert_eq!(tabs.list().len(), 1);
        assert_eq!(tabs.list()[0].url, "https://before.example");
    }

    #[test]
    fn tab_creation_rollback_removes_tab_and_empty_window() {
        let mut windows = WindowService::new();
        let window_id = windows.create_window(false);
        let mut tabs = TabService::new(window_id.clone());
        let tab = tabs.create_tab(window_id.clone(), "https://new.example".into(), true);
        windows.attach_tab(&window_id, &tab.id, true);
        let mut adapter = NoopEngineAdapter;
        let mut closed_tabs = Vec::new();
        let mut closed_windows = Vec::new();
        let mut context = RollbackContext {
            adapter: &mut adapter,
            tabs: &mut tabs,
            windows: &mut windows,
            closed_tabs: &mut closed_tabs,
            closed_windows: &mut closed_windows,
        };

        assert_eq!(
            context.apply(PendingOperation::TabCreated {
                tab_id: tab.id,
                window_id
            }),
            RollbackResult::Applied
        );
        assert!(tabs.list().is_empty());
        assert!(windows.list().is_empty());
    }
}
