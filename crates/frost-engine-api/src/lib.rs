use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("{0}")]
    Message(String),
}

pub type EngineResult<T> = Result<T, EngineError>;

pub trait EngineAdapter {
    fn create_page(
        &mut self,
        tab_id: &str,
        window_id: &str,
        url: &str,
        active: bool,
    ) -> EngineResult<()>;
    fn close_page(&mut self, tab_id: &str, successor_tab_id: Option<&str>) -> EngineResult<()>;
    fn navigate(&mut self, tab_id: &str, input: &str) -> EngineResult<()>;
    fn reload(&mut self, tab_id: &str) -> EngineResult<()>;
    fn stop(&mut self, tab_id: &str) -> EngineResult<()>;
    fn go_back(&mut self, tab_id: &str) -> EngineResult<()>;
    fn go_forward(&mut self, tab_id: &str) -> EngineResult<()>;
    fn create_window(&mut self, window_id: &str, is_private: bool) -> EngineResult<()>;
    fn close_window(&mut self, window_id: &str) -> EngineResult<()>;
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
    fn create_page(&mut self, _: &str, _: &str, _: &str, _: bool) -> EngineResult<()> {
        Ok(())
    }

    fn close_page(&mut self, _: &str, _: Option<&str>) -> EngineResult<()> {
        Ok(())
    }

    fn navigate(&mut self, _: &str, _: &str) -> EngineResult<()> {
        Ok(())
    }

    fn reload(&mut self, _: &str) -> EngineResult<()> {
        Ok(())
    }

    fn stop(&mut self, _: &str) -> EngineResult<()> {
        Ok(())
    }

    fn go_back(&mut self, _: &str) -> EngineResult<()> {
        Ok(())
    }

    fn go_forward(&mut self, _: &str) -> EngineResult<()> {
        Ok(())
    }

    fn create_window(&mut self, _: &str, _: bool) -> EngineResult<()> {
        Ok(())
    }

    fn close_window(&mut self, _: &str) -> EngineResult<()> {
        Ok(())
    }
}
