pub mod event;
pub mod request;
pub mod response;
pub mod state;

pub use event::{Event, EventEnvelope, SettingChanged, TabClosed, TabPatch};
pub use request::{ProtocolRequest, Request};
pub use response::{ProtocolResponse, Response};
pub use state::{AppState, BookmarkRecord, DownloadRecord, HistoryRecord, TabState, WindowState};

pub const PROTOCOL_VERSION: u16 = 0;
