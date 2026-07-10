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

    pub fn replace_all(&mut self, tabs: Vec<TabState>) {
        self.tabs = tabs;
    }

    pub fn upsert_tab(&mut self, tab: TabState) {
        if let Some(existing) = self.tabs.iter_mut().find(|t| t.id == tab.id) {
            *existing = tab;
        } else {
            self.tabs.push(tab);
        }
    }

    pub fn contains(&self, tab_id: &str) -> bool {
        self.tabs.iter().any(|t| t.id == tab_id)
    }

    pub fn create_tab(&mut self, window_id: String, url: String, active: bool) -> TabState {
        if active || self.tabs.is_empty() {
            self.tabs
                .iter_mut()
                .filter(|t| t.window_id == window_id)
                .for_each(|t| t.is_active = false);
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
            .find(|t| t.id == tab_id)
            .map(|t| t.window_id.clone())
        else {
            return false;
        };

        // First, deactivate the currently active tab in this window
        if let Some(current_active) = self
            .tabs
            .iter_mut()
            .find(|t| t.is_active && t.window_id == window_id)
        {
            current_active.is_active = false;
        }

        // Then activate the target tab
        if let Some(target) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            target.is_active = true;
        }
        true
    }

    pub fn close_tab(&mut self, tab_id: &str) -> Option<Option<String>> {
        let index = self.tabs.iter().position(|t| t.id == tab_id)?;
        let was_active = self.tabs[index].is_active;
        let window_id = self.tabs[index].window_id.clone();
        self.tabs.remove(index);

        let successor = if was_active {
            // Prefer the tab that shifted into the closed tab's position (the
            // right neighbour), falling back to the previous tab at the end.
            let window_indices: Vec<usize> = self
                .tabs
                .iter()
                .enumerate()
                .filter_map(|(index, tab)| (tab.window_id == window_id).then_some(index))
                .collect();
            let successor_index = window_indices
                .iter()
                .copied()
                .find(|candidate| *candidate >= index)
                .or_else(|| window_indices.last().copied());
            successor_index.map(|successor_index| {
                self.tabs[successor_index].is_active = true;
                self.tabs[successor_index].id.clone()
            })
        } else {
            None
        };
        Some(successor)
    }

    #[cfg(test)]
    fn tab_ids(&self) -> Vec<String> {
        self.tabs.iter().map(|tab| tab.id.clone()).collect()
    }
}

impl TabService {
    pub fn remove_tab(&mut self, tab_id: &str) -> Option<TabState> {
        let index = self.tabs.iter().position(|t| t.id == tab_id)?;
        Some(self.tabs.remove(index))
    }

    pub fn get_tab(&self, tab_id: &str) -> Option<TabState> {
        self.tabs.iter().find(|t| t.id == tab_id).cloned()
    }

    pub fn active_tab(&self) -> Option<TabState> {
        self.tabs.iter().find(|t| t.is_active).cloned()
    }

    pub fn has_tabs_in_window(&self, window_id: &str) -> bool {
        self.tabs.iter().any(|tab| tab.window_id == window_id)
    }

    pub fn pin_tab(&mut self, tab_id: &str, pinned: bool) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.is_pinned = pinned;
        true
    }

    pub fn duplicate_tab(&mut self, tab_id: &str) -> Option<TabState> {
        let mut tab = self.get_tab(tab_id)?;
        tab.id = format!("tab-{}", Uuid::new_v4());
        tab.is_active = false;
        self.tabs.push(tab.clone());
        Some(tab)
    }

    pub fn close_other_tabs(&mut self, tab_id: &str) -> Vec<TabState> {
        let Some(window_id) = self
            .tabs
            .iter()
            .find(|t| t.id == tab_id)
            .map(|t| t.window_id.clone())
        else {
            return Vec::new();
        };
        let mut closed = Vec::new();
        self.tabs.retain(|tab| {
            let keep = tab.id == tab_id || tab.is_pinned || tab.window_id != window_id;
            if !keep {
                closed.push(tab.clone());
            }
            keep
        });
        self.activate_tab(tab_id);
        closed
    }

    pub fn close_tabs_to_right(&mut self, tab_id: &str) -> Vec<TabState> {
        let Some(window_id) = self
            .tabs
            .iter()
            .find(|t| t.id == tab_id)
            .map(|t| t.window_id.clone())
        else {
            return Vec::new();
        };
        // Collect closed tabs using a filter-based approach to avoid
        // complex index arithmetic and potential panics.
        let mut found_target = false;
        let mut closed = Vec::new();
        self.tabs.retain(|tab| {
            if tab.window_id != window_id {
                return true;
            }
            if tab.id == tab_id {
                found_target = true;
                return true;
            }
            if !found_target || tab.is_pinned {
                return true;
            }
            closed.push(tab.clone());
            false
        });
        closed
    }

    pub fn move_tab(&mut self, tab_id: &str, to_index: usize) -> bool {
        let Some(index) = self.tabs.iter().position(|t| t.id == tab_id) else {
            return false;
        };
        let window_id = self.tabs[index].window_id.clone();
        // Collect indices of tabs in the same window.
        let window_indices: Vec<usize> = self
            .tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| t.window_id == window_id)
            .map(|(i, _)| i)
            .collect();
        let Some(local_pos) = window_indices.iter().position(|&i| i == index) else {
            return false;
        };
        let local_to = to_index.min(window_indices.len().saturating_sub(1));
        if local_pos == local_to {
            return true; // no-op
        }
        // Remove the tab and calculate positions again after removal. Original
        // indices cannot be reused here because every item after `index`
        // shifts left (which previously made forward moves land one slot late).
        let tab = self.tabs.remove(index);
        let window_indices_after: Vec<usize> = self
            .tabs
            .iter()
            .enumerate()
            .filter(|(_, candidate)| candidate.window_id == window_id)
            .map(|(index, _)| index)
            .collect();
        let insert_at = if local_to >= window_indices_after.len() {
            window_indices_after
                .last()
                .map_or(self.tabs.len(), |index| index + 1)
        } else {
            window_indices_after[local_to]
        };
        self.tabs.insert(insert_at, tab);
        true
    }

    pub fn move_tab_to_window(&mut self, tab_id: &str, window_id: &str) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.window_id = window_id.to_owned();
        tab.is_active = false;
        // Deactivate all tabs in the target window, then activate only this one.
        for t in self.tabs.iter_mut() {
            if t.window_id == window_id && t.id != tab_id {
                t.is_active = false;
            }
        }
        // The tab must exist since we just modified it above.
        if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) {
            tab.is_active = true;
        }
        true
    }

    pub fn stop_tab(&mut self, tab_id: &str) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.is_loading = false;
        true
    }

    pub fn navigate(&mut self, tab_id: &str, input: &str) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.url = input.to_owned();
        tab.error_text.clear();
        tab.is_loading = true;
        true
    }

    pub fn set_title(&mut self, tab_id: &str, title: &str) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.title = title.to_owned();
        true
    }

    pub fn set_url(&mut self, tab_id: &str, url: &str) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.url = url.to_owned();
        true
    }

    pub fn set_favicon_url(&mut self, tab_id: &str, favicon_url: &str) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.favicon_url = favicon_url.to_owned();
        true
    }

    pub fn set_loading(&mut self, tab_id: &str, is_loading: bool) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.is_loading = is_loading;
        true
    }

    pub fn set_navigation_state(
        &mut self,
        tab_id: &str,
        can_go_back: bool,
        can_go_forward: bool,
    ) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.can_go_back = can_go_back;
        tab.can_go_forward = can_go_forward;
        true
    }

    pub fn set_error_text(&mut self, tab_id: &str, error_text: &str) -> bool {
        let Some(tab) = self.tabs.iter_mut().find(|t| t.id == tab_id) else {
            return false;
        };
        tab.error_text = error_text.to_owned();
        tab.is_loading = false;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn three_tabs() -> (TabService, Vec<String>) {
        let mut tabs = TabService::new("window-1".into());
        tabs.create_tab("window-1".into(), "a".into(), true);
        tabs.create_tab("window-1".into(), "b".into(), true);
        tabs.create_tab("window-1".into(), "c".into(), true);
        let ids = tabs.tab_ids();
        (tabs, ids)
    }

    #[test]
    fn close_active_middle_prefers_right_neighbour() {
        let (mut tabs, ids) = three_tabs();
        tabs.activate_tab(&ids[1]);
        assert_eq!(tabs.close_tab(&ids[1]), Some(Some(ids[2].clone())));
    }

    #[test]
    fn close_active_end_falls_back_to_left_neighbour() {
        let (mut tabs, ids) = three_tabs();
        tabs.activate_tab(&ids[2]);
        assert_eq!(tabs.close_tab(&ids[2]), Some(Some(ids[1].clone())));
    }

    #[test]
    fn close_active_first_prefers_right_neighbour() {
        let (mut tabs, ids) = three_tabs();
        tabs.activate_tab(&ids[0]);
        assert_eq!(tabs.close_tab(&ids[0]), Some(Some(ids[1].clone())));
    }
}
