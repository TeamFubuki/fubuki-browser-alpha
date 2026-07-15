use frost_protocol::WindowState;
use uuid::Uuid;

pub struct WindowService {
    windows: Vec<WindowState>,
    active_window_id: Option<String>,
}

impl WindowService {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            active_window_id: None,
        }
    }

    pub fn create_window(&mut self, is_private: bool) -> String {
        let id = format!("window-{}", Uuid::new_v4());
        self.windows.push(WindowState {
            id: id.clone(),
            active_tab_id: None,
            is_private,
            tab_ids: Vec::new(),
        });
        self.active_window_id = Some(id.clone());
        id
    }

    /// Registers a window created by the host during startup/session restore.
    /// The initial empty placeholder is adopted instead of leaving a phantom
    /// window in snapshots.
    pub fn ensure_window(&mut self, window_id: &str, is_private: bool) {
        if let Some(window) = self.windows.iter_mut().find(|w| w.id == window_id) {
            // Preserve existing private flag - don't overwrite with false
            window.is_private = window.is_private || is_private;
            self.active_window_id = Some(window_id.to_owned());
            return;
        }
        if self.windows.len() == 1 && self.windows[0].tab_ids.is_empty() {
            self.windows[0].id = window_id.to_owned();
            self.windows[0].is_private = is_private;
        } else {
            self.windows.push(WindowState {
                id: window_id.to_owned(),
                active_tab_id: None,
                is_private,
                tab_ids: Vec::new(),
            });
        }
        self.active_window_id = Some(window_id.to_owned());
    }

    pub fn close_window(&mut self, window_id: &str) -> bool {
        let Some(index) = self.windows.iter().position(|w| w.id == window_id) else {
            return false;
        };
        self.windows.remove(index);
        if self.active_window_id.as_deref() == Some(window_id) {
            self.active_window_id = self.windows.last().map(|w| w.id.clone());
        }
        true
    }

    pub fn replace_all(&mut self, windows: Vec<WindowState>, active_window_id: Option<String>) {
        self.windows = windows;
        self.active_window_id =
            active_window_id.or_else(|| self.windows.last().map(|w| w.id.clone()));
    }

    pub fn list(&self) -> Vec<WindowState> {
        self.windows.clone()
    }

    pub fn get_window(&self, window_id: &str) -> Option<WindowState> {
        self.windows.iter().find(|w| w.id == window_id).cloned()
    }

    pub fn active_window_id(&self) -> Option<&str> {
        self.active_window_id.as_deref()
    }

    pub fn set_active_window(&mut self, window_id: &str) -> bool {
        if self.windows.iter().any(|w| w.id == window_id) {
            self.active_window_id = Some(window_id.to_owned());
            return true;
        }
        false
    }

    pub fn attach_tab(&mut self, window_id: &str, tab_id: &str, activate: bool) {
        if let Some(window) = self.windows.iter_mut().find(|w| w.id == window_id) {
            if !window.tab_ids.iter().any(|id| id == tab_id) {
                window.tab_ids.push(tab_id.to_owned());
            }
            if activate {
                window.active_tab_id = Some(tab_id.to_owned());
            }
        }
    }

    pub fn detach_tab(&mut self, tab_id: &str) {
        for window in &mut self.windows {
            if let Some(index) = window.tab_ids.iter().position(|id| id == tab_id) {
                window.tab_ids.remove(index);
                if window.active_tab_id.as_deref() == Some(tab_id) {
                    window.active_tab_id = window.tab_ids.last().cloned();
                }
                break;
            }
        }
    }

    pub fn move_tab_to_window(&mut self, tab_id: &str, window_id: &str) {
        self.detach_tab(tab_id);
        self.attach_tab(window_id, tab_id, true);
    }

    pub fn set_active_tab(&mut self, tab_id: &str) {
        for window in &mut self.windows {
            if window.tab_ids.iter().any(|id| id == tab_id) {
                window.active_tab_id = Some(tab_id.to_owned());
                self.active_window_id = Some(window.id.clone());
                return;
            }
        }
    }
}

impl Default for WindowService {
    fn default() -> Self {
        Self::new()
    }
}
