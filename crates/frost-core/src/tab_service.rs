use frost_protocol::TabState;
use uuid::Uuid;

pub struct TabService {
    tabs: Vec<TabState>,
    default_window_id: String,
}

impl TabService {
    pub fn new(default_window_id: String) -> Self {
        Self {
            tabs: Vec::new(),
            default_window_id,
        }
    }

    pub fn list(&self) -> Vec<TabState> {
        self.tabs.clone()
    }

    pub fn contains(&self, tab_id: &str) -> bool {
        self.tabs.iter().any(|tab| tab.id == tab_id)
    }

    pub fn create_tab(&mut self, window_id: String, url: String, active: bool) -> TabState {
        if active || self.tabs.is_empty() {
            self.tabs
                .iter_mut()
                .filter(|tab| tab.window_id == window_id)
                .for_each(|tab| tab.is_active = false);
        }

        let tab = TabState {
            id: format!("tab-{}", Uuid::new_v4()),
            window_id: if window_id.is_empty() {
                self.default_window_id.clone()
            } else {
                window_id
            },
            title: "New Tab".into(),
            url,
            favicon_url: String::new(),
            error_text: String::new(),
            zoom_level: 0.0,
            is_loading: false,
            can_go_back: false,
            can_go_forward: false,
            is_active: active || self.tabs.is_empty(),
            is_pinned: false,
        };
        self.tabs.push(tab.clone());
        tab
    }

    pub fn activate_tab(&mut self, tab_id: &str) -> bool {
        let Some(window_id) = self
            .tabs
            .iter()
            .find(|tab| tab.id == tab_id)
            .map(|tab| tab.window_id.clone())
        else {
            return false;
        };

        for tab in &mut self.tabs {
            if tab.window_id == window_id {
                tab.is_active = tab.id == tab_id;
            }
        }
        true
    }

    pub fn close_tab(&mut self, tab_id: &str) -> bool {
        let Some(index) = self.tabs.iter().position(|tab| tab.id == tab_id) else {
            return false;
        };
        let was_active = self.tabs[index].is_active;
        let window_id = self.tabs[index].window_id.clone();
        self.tabs.remove(index);

        if was_active {
            if let Some(next) = self.tabs.iter_mut().find(|tab| tab.window_id == window_id) {
                next.is_active = true;
            }
        }
        true
    }

    pub fn navigate(&mut self, tab_id: &str, input: &str) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return false;
        };
        tab.url = input.to_owned();
        tab.error_text.clear();
        tab.is_loading = true;
        true
    }
}
