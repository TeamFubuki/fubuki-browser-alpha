use serde::{Deserialize, Serialize};

/// Permission kinds understood by the Frost permission broker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionType {
    Camera,
    Microphone,
    Geolocation,
    Notifications,
    PointerLock,
    KeyboardLock,
}

impl PermissionType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Camera => "camera",
            Self::Microphone => "microphone",
            Self::Geolocation => "geolocation",
            Self::Notifications => "notifications",
            Self::PointerLock => "pointerLock",
            Self::KeyboardLock => "keyboardLock",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "camera" => Some(Self::Camera),
            "microphone" => Some(Self::Microphone),
            "geolocation" => Some(Self::Geolocation),
            "notifications" => Some(Self::Notifications),
            "pointerLock" => Some(Self::PointerLock),
            "keyboardLock" => Some(Self::KeyboardLock),
            _ => None,
        }
    }
}

/// A persisted permission decision. Missing records are interpreted as ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionDecision {
    #[serde(rename = "ask")]
    Ask,
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "block", alias = "deny")]
    Block,
}

impl PermissionDecision {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ask => "ask",
            Self::Allow => "allow",
            Self::Block => "block",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "ask" => Some(Self::Ask),
            "allow" => Some(Self::Allow),
            "block" | "deny" => Some(Self::Block),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_wire_names_are_canonical() {
        assert_eq!(
            serde_json::to_string(&PermissionType::PointerLock).unwrap(),
            "\"pointerLock\""
        );
        assert_eq!(
            serde_json::to_string(&PermissionDecision::Block).unwrap(),
            "\"block\""
        );
    }

    #[test]
    fn legacy_deny_is_read_as_block() {
        assert_eq!(
            serde_json::from_str::<PermissionDecision>("\"deny\"").unwrap(),
            PermissionDecision::Block
        );
        assert_eq!(
            PermissionDecision::parse("deny"),
            Some(PermissionDecision::Block)
        );
    }
}
