use frost_protocol::{PermissionDecision, PermissionType};
use frost_store::{PermissionRepository, StoreError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("permission origin must be an HTTP(S) origin")]
    InvalidOrigin,
    #[error("unsupported permission type: {0}")]
    UnsupportedType(String),
    #[error("unsupported permission decision: {0}")]
    UnsupportedDecision(String),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Engine-owned permission policy and origin normalization boundary.
pub struct PermissionService;

impl PermissionService {
    pub fn lookup<S: PermissionRepository>(
        repository: &S,
        origin: &str,
        permission: PermissionType,
    ) -> Result<PermissionDecision, PermissionError> {
        let origin = Self::normalize_origin(origin)?;
        let record = repository.get_permission(&origin, permission.as_str())?;
        match record {
            None => Ok(PermissionDecision::Ask),
            Some(record) => PermissionDecision::parse(&record.value)
                .ok_or(PermissionError::UnsupportedDecision(record.value)),
        }
    }

    pub fn set<S: PermissionRepository>(
        repository: &S,
        origin: &str,
        permission: PermissionType,
        decision: PermissionDecision,
    ) -> Result<String, PermissionError> {
        let origin = Self::normalize_origin(origin)?;
        match decision {
            PermissionDecision::Ask => {
                repository.remove_permission(&origin, permission.as_str())?;
            }
            PermissionDecision::Allow | PermissionDecision::Block => {
                repository.set_permission(&origin, permission.as_str(), decision.as_str())?;
            }
        }
        Ok(origin)
    }

    pub fn set_value<S: PermissionRepository>(
        repository: &S,
        origin: &str,
        permission: &str,
        value: &str,
    ) -> Result<String, PermissionError> {
        let permission = PermissionType::parse(permission)
            .ok_or_else(|| PermissionError::UnsupportedType(permission.to_owned()))?;
        let decision = PermissionDecision::parse(value)
            .ok_or_else(|| PermissionError::UnsupportedDecision(value.to_owned()))?;
        Self::set(repository, origin, permission, decision)
    }

    pub fn normalize_origin(input: &str) -> Result<String, PermissionError> {
        let input = input.trim();
        let (scheme, remainder) = input
            .split_once("://")
            .ok_or(PermissionError::InvalidOrigin)?;
        let scheme = scheme.to_ascii_lowercase();
        if scheme != "http" && scheme != "https" {
            return Err(PermissionError::InvalidOrigin);
        }

        let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
        let authority = &remainder[..authority_end];
        if authority.is_empty() || authority.contains('@') {
            return Err(PermissionError::InvalidOrigin);
        }

        let (host, port) = if authority.starts_with('[') {
            let close = authority.find(']').ok_or(PermissionError::InvalidOrigin)?;
            let suffix = &authority[close + 1..];
            let port = if suffix.is_empty() {
                None
            } else {
                Some(
                    suffix
                        .strip_prefix(':')
                        .ok_or(PermissionError::InvalidOrigin)?,
                )
            };
            (&authority[..=close], port)
        } else {
            if authority.matches(':').count() > 1 {
                return Err(PermissionError::InvalidOrigin);
            }
            match authority.rsplit_once(':') {
                Some((host, port)) => (host, Some(port)),
                None => (authority, None),
            }
        };

        if host.is_empty()
            || host
                .chars()
                .any(|character| character.is_ascii_control() || character.is_whitespace())
        {
            return Err(PermissionError::InvalidOrigin);
        }
        if let Some(port) = port {
            if port.is_empty() || !port.chars().all(|character| character.is_ascii_digit()) {
                return Err(PermissionError::InvalidOrigin);
            }
            let port_number = port
                .parse::<u16>()
                .map_err(|_| PermissionError::InvalidOrigin)?;
            let is_default = (scheme == "http" && port_number == 80)
                || (scheme == "https" && port_number == 443);
            let host = host.to_ascii_lowercase();
            return Ok(if is_default {
                format!("{scheme}://{host}")
            } else {
                format!("{scheme}://{host}:{port_number}")
            });
        }

        Ok(format!("{scheme}://{}", host.to_ascii_lowercase()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InMemoryStore;

    #[test]
    fn normalizes_origins_to_scheme_host_and_non_default_port() {
        assert_eq!(
            PermissionService::normalize_origin(" HTTPS://Example.COM:443/path ").unwrap(),
            "https://example.com"
        );
        assert_eq!(
            PermissionService::normalize_origin("http://Example.COM:8080/path").unwrap(),
            "http://example.com:8080"
        );
    }

    #[test]
    fn rejects_internal_and_credential_origins() {
        assert!(PermissionService::normalize_origin("fubuki://settings/").is_err());
        assert!(PermissionService::normalize_origin("https://user@example.com").is_err());
    }

    #[test]
    fn missing_permission_is_ask_and_ask_removes_persistence() {
        let store = InMemoryStore::default();
        assert_eq!(
            PermissionService::lookup(&store, "https://EXAMPLE.com/", PermissionType::Camera)
                .unwrap(),
            PermissionDecision::Ask
        );
        PermissionService::set(
            &store,
            "https://example.com",
            PermissionType::Camera,
            PermissionDecision::Allow,
        )
        .unwrap();
        assert_eq!(store.list_permissions().unwrap().len(), 1);
        PermissionService::set(
            &store,
            "https://example.com",
            PermissionType::Camera,
            PermissionDecision::Ask,
        )
        .unwrap();
        assert!(store.list_permissions().unwrap().is_empty());
    }
}
