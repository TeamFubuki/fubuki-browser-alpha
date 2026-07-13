use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("{0}")]
    Message(String),
}

pub type EngineResult<T> = Result<T, EngineError>;
pub type HostCommandId = Option<String>;

pub trait EngineAdapter {
    fn create_page(
        &mut self,
        tab_id: &str,
        window_id: &str,
        url: &str,
        active: bool,
    ) -> EngineResult<HostCommandId>;
    fn close_page(&mut self, tab_id: &str) -> EngineResult<HostCommandId>;
    fn activate_page(&mut self, tab_id: &str) -> EngineResult<HostCommandId>;
    fn set_page_pinned(&mut self, tab_id: &str, pinned: bool) -> EngineResult<HostCommandId>;
    fn move_page(&mut self, tab_id: &str, to_index: usize) -> EngineResult<HostCommandId>;
    fn navigate(&mut self, tab_id: &str, input: &str) -> EngineResult<HostCommandId>;
    fn reload(&mut self, tab_id: &str) -> EngineResult<HostCommandId>;
    fn stop(&mut self, tab_id: &str) -> EngineResult<HostCommandId>;
    fn go_back(&mut self, tab_id: &str) -> EngineResult<HostCommandId>;
    fn go_forward(&mut self, tab_id: &str) -> EngineResult<HostCommandId>;
    fn create_window(&mut self, window_id: &str, is_private: bool) -> EngineResult<HostCommandId>;
    fn close_window(&mut self, window_id: &str) -> EngineResult<HostCommandId>;
    fn open_file(&mut self, path: &str) -> EngineResult<HostCommandId>;
    fn reveal_file(&mut self, path: &str) -> EngineResult<HostCommandId>;
}

pub trait PageAdapter {
    fn title_changed(&mut self, tab_id: &str, title: &str) -> EngineResult<()>;
    fn url_changed(&mut self, tab_id: &str, url: &str) -> EngineResult<()>;
    fn loading_changed(&mut self, tab_id: &str, is_loading: bool) -> EngineResult<()>;
    fn navigation_state_changed(
        &mut self,
        tab_id: &str,
        can_go_back: bool,
        can_go_forward: bool,
    ) -> EngineResult<()>;
}

pub trait WindowHost {
    fn show_window(&mut self, window_id: &str) -> EngineResult<()>;
    fn focus_window(&mut self, window_id: &str) -> EngineResult<()>;
}

#[derive(Debug, Default)]
pub struct NoopEngineAdapter;

impl EngineAdapter for NoopEngineAdapter {
    fn create_page(&mut self, _: &str, _: &str, _: &str, _: bool) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn close_page(&mut self, _: &str) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn activate_page(&mut self, _: &str) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn set_page_pinned(&mut self, _: &str, _: bool) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn move_page(&mut self, _: &str, _: usize) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn navigate(&mut self, _: &str, _: &str) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn reload(&mut self, _: &str) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn stop(&mut self, _: &str) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn go_back(&mut self, _: &str) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn go_forward(&mut self, _: &str) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn create_window(&mut self, _: &str, _: bool) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn close_window(&mut self, _: &str) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn open_file(&mut self, _: &str) -> EngineResult<HostCommandId> {
        Ok(None)
    }

    fn reveal_file(&mut self, _: &str) -> EngineResult<HostCommandId> {
        Ok(None)
    }
}
