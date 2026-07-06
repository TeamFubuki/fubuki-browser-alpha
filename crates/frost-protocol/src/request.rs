use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolRequest {
    #[serde(default = "default_version")]
    pub version: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(flatten)]
    pub request: Request,
}

fn default_version() -> u16 {
    crate::PROTOCOL_VERSION
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum Request {
    #[serde(rename = "app.snapshot")]
    AppSnapshot,
    #[serde(rename = "tabs.list")]
    TabsList,
    #[serde(rename = "tabs.create", rename_all = "camelCase")]
    TabsCreate { url: Option<String>, active: bool },
    #[serde(rename = "tabs.activate", rename_all = "camelCase")]
    TabsActivate { tab_id: String },
    #[serde(rename = "tabs.close", rename_all = "camelCase")]
    TabsClose { tab_id: String },
    #[serde(rename = "tabs.navigate", rename_all = "camelCase")]
    TabsNavigate { tab_id: String, input: String },
    #[serde(rename = "tabs.reload", rename_all = "camelCase")]
    TabsReload { tab_id: String },
    #[serde(rename = "tabs.goBack", rename_all = "camelCase")]
    TabsGoBack { tab_id: String },
    #[serde(rename = "tabs.goForward", rename_all = "camelCase")]
    TabsGoForward { tab_id: String },
    #[serde(rename = "windows.list")]
    WindowsList,
    #[serde(rename = "windows.create")]
    WindowsCreate,
    #[serde(rename = "windows.close", rename_all = "camelCase")]
    WindowsClose { window_id: Option<String> },
    #[serde(rename = "settings.get", rename_all = "camelCase")]
    SettingsGet { key: String },
    #[serde(rename = "settings.set", rename_all = "camelCase")]
    SettingsSet { key: String, value: String },
}

impl ProtocolRequest {
    pub fn new(request: Request) -> Self {
        Self {
            version: crate::PROTOCOL_VERSION,
            id: None,
            request,
        }
    }
}
