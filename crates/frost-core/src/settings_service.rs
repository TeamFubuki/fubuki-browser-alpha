use std::path::PathBuf;

use frost_store::{SettingsRepository, StoreError, StoreResult};

pub struct SettingsService;

impl SettingsService {
    /// Valid setting keys that can be stored and retrieved.
    pub const VALID_KEYS: &[&str] = &[
        "homepage",
        "searchEngine",
        "customSearchUrl",
        "startupBehavior",
        "sessionJson",
        "downloadDirectory",
        "theme",
        "appearance",
        "toolbarDensity",
        "sidebarVisible",
        "sidebarWidth",
        "defaultBookmarkDisplay",
        "openBookmarkIn",
        "showBookmarkFavicons",
        "newTabPage",
        "homeUrl",
        "askBeforeDownload",
        "language",
        "defaultZoomLevel",
        "closeWindowWithLastTab",
        "privateSearchEngine",
        "newTabBackgroundMode",
        "newTabBackgroundColor",
        "newTabBackgroundUrl",
    ];

    /// Returns the default value for a given setting key.
    pub fn default_value(key: &str) -> String {
        match key {
            "homepage" | "homeUrl" => "https://example.com".into(),
            "searchEngine" => "google".into(),
            "customSearchUrl" => "https://www.google.com/search?q={query}".into(),
            "startupBehavior" => "newTab".into(),
            "sessionJson" => String::new(),
            "downloadDirectory" => default_download_directory(),
            "theme" => "light".into(),
            "appearance" => "system".into(),
            "toolbarDensity" => "compact".into(),
            "sidebarVisible" => "show".into(),
            "sidebarWidth" => "196".into(),
            "defaultBookmarkDisplay" => "sidebar".into(),
            "openBookmarkIn" => "current".into(),
            "showBookmarkFavicons" => "on".into(),
            "newTabPage" => "blank".into(),
            "askBeforeDownload" => "off".into(),
            "language" => "system".into(),
            "defaultZoomLevel" => "0".into(),
            "closeWindowWithLastTab" => "off".into(),
            "privateSearchEngine" => "default".into(),
            "newTabBackgroundMode" => "unsplash".into(),
            "newTabBackgroundColor" => "#f8fafd".into(),
            "newTabBackgroundUrl" => String::new(),
            _ => String::new(),
        }
    }

    pub fn get<S: SettingsRepository>(repository: &S, key: &str) -> StoreResult<Option<String>> {
        if !Self::VALID_KEYS.contains(&key) {
            return Err(StoreError::InvalidKey(key.to_owned()));
        }
        repository.get_setting(key)
    }

    pub fn set<S: SettingsRepository>(repository: &S, key: &str, value: &str) -> StoreResult<()> {
        if !Self::VALID_KEYS.contains(&key) {
            return Err(StoreError::InvalidKey(key.to_owned()));
        }
        Self::validate(key, value)?;
        repository.set_setting(key, value)
    }

    fn validate(key: &str, value: &str) -> StoreResult<()> {
        let enum_value = |allowed: &[&str]| {
            if allowed.contains(&value) {
                Ok(())
            } else {
                Err(StoreError::InvalidKey(format!(
                    "Invalid value for {key}: {value}"
                )))
            }
        };

        match key {
            "searchEngine" => enum_value(&["google", "duckduckgo", "bing", "custom"]),
            "startupBehavior" => enum_value(&["newTab", "restore", "homePage"]),
            "theme" => enum_value(&["light", "dark"]),
            "appearance" => enum_value(&["system", "light", "dark"]),
            "toolbarDensity" => enum_value(&["compact", "comfortable"]),
            "sidebarVisible" => enum_value(&["show", "hide"]),
            "defaultBookmarkDisplay" => enum_value(&["sidebar", "bar"]),
            "openBookmarkIn" => enum_value(&["current", "newTab"]),
            "showBookmarkFavicons" | "askBeforeDownload" | "closeWindowWithLastTab" => {
                enum_value(&["on", "off"])
            }
            "newTabPage" => enum_value(&["blank", "home"]),
            "language" => enum_value(&["system", "en", "ja"]),
            "privateSearchEngine" => enum_value(&["default", "google", "duckduckgo", "bing"]),
            "newTabBackgroundMode" => enum_value(&["unsplash", "color", "image"]),
            "sidebarWidth" => validate_number(key, value, 160.0, 640.0),
            "defaultZoomLevel" => validate_number(key, value, -5.0, 5.0),
            "homepage" | "homeUrl" | "newTabBackgroundUrl" => validate_url(key, value, true),
            "customSearchUrl" => {
                validate_url(key, value, false)?;
                if value.contains("{query}") {
                    Ok(())
                } else {
                    Err(StoreError::InvalidKey(format!(
                        "Invalid value for {key}: URL must contain {{query}}"
                    )))
                }
            }
            "downloadDirectory" => {
                if value.is_empty() || !PathBuf::from(value).is_absolute() {
                    Err(StoreError::InvalidKey(format!(
                        "Invalid value for {key}: an absolute path is required"
                    )))
                } else {
                    Ok(())
                }
            }
            "newTabBackgroundColor" => {
                if is_hex_color(value) {
                    Ok(())
                } else {
                    Err(StoreError::InvalidKey(format!(
                        "Invalid value for {key}: expected #RRGGBB or #RGB"
                    )))
                }
            }
            "sessionJson" => {
                if value.is_empty() || serde_json::from_str::<serde_json::Value>(value).is_ok() {
                    Ok(())
                } else {
                    Err(StoreError::InvalidKey(format!(
                        "Invalid value for {key}: expected JSON"
                    )))
                }
            }
            _ => Ok(()),
        }
    }
}

fn default_download_directory() -> String {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| home.join("Downloads"))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .to_string_lossy()
        .into_owned()
}

fn validate_number(key: &str, value: &str, min: f64, max: f64) -> StoreResult<()> {
    match value.parse::<f64>() {
        Ok(number) if number.is_finite() && (min..=max).contains(&number) => Ok(()),
        _ => Err(StoreError::InvalidKey(format!(
            "Invalid value for {key}: expected a number between {min} and {max}"
        ))),
    }
}

fn validate_url(key: &str, value: &str, allow_empty: bool) -> StoreResult<()> {
    if value.is_empty() && allow_empty {
        return Ok(());
    }
    if value.starts_with("https://")
        || value.starts_with("http://")
        || value.starts_with("fubuki://")
    {
        Ok(())
    } else {
        Err(StoreError::InvalidKey(format!(
            "Invalid value for {key}: expected an http(s) or fubuki URL"
        )))
    }
}

fn is_hex_color(value: &str) -> bool {
    let hex = value.strip_prefix('#').unwrap_or_default();
    matches!(hex.len(), 3 | 6) && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owns_the_full_settings_catalog_and_defaults() {
        for key in SettingsService::VALID_KEYS {
            let default = SettingsService::default_value(key);
            assert!(SettingsService::validate(key, &default).is_ok(), "{key}");
        }
    }

    #[test]
    fn rejects_invalid_persisted_setting_values() {
        assert!(SettingsService::validate("askBeforeDownload", "maybe").is_err());
        assert!(SettingsService::validate("downloadDirectory", "relative").is_err());
        assert!(SettingsService::validate("customSearchUrl", "https://example.test/").is_err());
        assert!(SettingsService::validate("sidebarWidth", "NaN").is_err());
    }
}
