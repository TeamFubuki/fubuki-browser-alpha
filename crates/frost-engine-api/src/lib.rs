use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("{0}")]
    Message(String),
}

pub type EngineResult<T> = Result<T, EngineError>;

/// The outcome of handing a command to the host.
///
/// `Queued` is deliberately not success: the owning core must defer its state
/// mutation until a matching host result arrives. `Completed` is reserved for
/// in-process adapters used by tests and headless deployments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostDispatch {
    Completed,
    Queued { operation_id: String },
}

impl HostDispatch {
    pub fn operation_id(&self) -> Option<&str> {
        match self {
            Self::Completed => None,
            Self::Queued { operation_id } => Some(operation_id),
        }
    }
}

pub trait EngineAdapter {
    fn create_page(
        &mut self,
        tab_id: &str,
        window_id: &str,
        url: &str,
    ) -> EngineResult<HostDispatch>;
    fn close_page(&mut self, tab_id: &str) -> EngineResult<HostDispatch>;
    fn navigate(&mut self, tab_id: &str, input: &str) -> EngineResult<HostDispatch>;
    fn reload(&mut self, tab_id: &str) -> EngineResult<HostDispatch>;
    fn stop(&mut self, tab_id: &str) -> EngineResult<HostDispatch>;
    fn go_back(&mut self, tab_id: &str) -> EngineResult<HostDispatch>;
    fn go_forward(&mut self, tab_id: &str) -> EngineResult<HostDispatch>;
    fn create_window(&mut self, window_id: &str, is_private: bool) -> EngineResult<HostDispatch>;
    fn close_window(&mut self, window_id: &str) -> EngineResult<HostDispatch>;
    /// Starts an isolated private runtime.  This must not create a window in
    /// the caller's persistent engine state.
    fn create_private_runtime(&mut self) -> EngineResult<HostDispatch>;
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
    fn create_page(&mut self, _: &str, _: &str, _: &str) -> EngineResult<HostDispatch> {
        Ok(HostDispatch::Completed)
    }

    fn close_page(&mut self, _: &str) -> EngineResult<HostDispatch> {
        Ok(HostDispatch::Completed)
    }

    fn navigate(&mut self, _: &str, _: &str) -> EngineResult<HostDispatch> {
        Ok(HostDispatch::Completed)
    }

    fn reload(&mut self, _: &str) -> EngineResult<HostDispatch> {
        Ok(HostDispatch::Completed)
    }

    fn stop(&mut self, _: &str) -> EngineResult<HostDispatch> {
        Ok(HostDispatch::Completed)
    }

    fn go_back(&mut self, _: &str) -> EngineResult<HostDispatch> {
        Ok(HostDispatch::Completed)
    }

    fn go_forward(&mut self, _: &str) -> EngineResult<HostDispatch> {
        Ok(HostDispatch::Completed)
    }

    fn create_window(&mut self, _: &str, _: bool) -> EngineResult<HostDispatch> {
        Ok(HostDispatch::Completed)
    }

    fn close_window(&mut self, _: &str) -> EngineResult<HostDispatch> {
        Ok(HostDispatch::Completed)
    }

    fn create_private_runtime(&mut self) -> EngineResult<HostDispatch> {
        Ok(HostDispatch::Completed)
    }
}
