pub mod event;
pub mod external;
pub mod host;
pub mod request;
pub mod response;
pub mod state;

pub use event::{
    Event, EventEnvelope, SettingChanged, TabActivated, TabClosed, TabMoved, TabPatch,
};
pub use external::{
    ExternalCapability, ExternalCommand, ExternalCommandEnvelope, ExternalEvent,
    ExternalEventEnvelope,
};
pub use host::{
    HostCommand, HostCommandEnvelope, HostCommandResultEnvelope, HostEvent, HostEventEnvelope,
};
pub use request::{ProtocolRequest, Request};
pub use response::{ProtocolResponse, Response};
pub use state::{
    AppState, BookmarkRecord, BrowserCommand, DownloadRecord, HistoryRecord, PermissionRecord,
    TabState, WindowState,
};

pub const PROTOCOL_VERSION: u16 = 0;
