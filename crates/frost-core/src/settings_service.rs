use frost_store::{SettingsRepository, StoreError, StoreResult};

pub struct SettingsService;

impl SettingsService {
    /// Valid setting keys that can be stored and retrieved.
    pub const VALID_KEYS: &[&str] = &[
        "homepage",
        "searchEngine",
        "customSearchUrl",
        "startupBehavior",
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
        "closeWindowWithLastTab",
        "privateSearchEngine",
        "language",
        "defaultZoomLevel",
        "newTabBackgroundMode",
        "newTabBackgroundColor",
        "newTabBackgroundUrl",
    ];

    /// Returns the default value for a given setting key.
    pub fn default_value(key: &str) -> &'static str {
        match key {
            "homepage" | "homeUrl" => "https://example.com",
            "searchEngine" => "google",
            "customSearchUrl" => "https://www.google.com/search?q={query}",
            "startupBehavior" => "newTab",
            "downloadDirectory" => "",
            "theme" => "light",
            "appearance" => "system",
            "toolbarDensity" => "compact",
            "sidebarVisible" => "show",
            "sidebarWidth" => "196",
            "defaultBookmarkDisplay" => "sidebar",
            "openBookmarkIn" => "current",
            "showBookmarkFavicons" => "on",
            "newTabPage" => "blank",
            "askBeforeDownload" | "closeWindowWithLastTab" => "off",
            "privateSearchEngine" => "default",
            "language" => "system",
            "defaultZoomLevel" => "0",
            "newTabBackgroundMode" => "unsplash",
            "newTabBackgroundColor" => "#f8fafd",
            "newTabBackgroundUrl" => "",
            _ => "",
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
        if !Self::valid_value(key, value) {
            return Err(StoreError::InvalidValue {
                key: key.to_owned(),
                value: value.to_owned(),
            });
        }
        repository.set_setting(key, value)
    }

    fn valid_value(key: &str, value: &str) -> bool {
        match key {
            "searchEngine" => matches!(value, "google" | "duckduckgo" | "bing" | "custom"),
            "startupBehavior" => matches!(value, "newTab" | "homePage" | "restore"),
            "theme" => matches!(value, "light" | "dark"),
            "appearance" => matches!(value, "system" | "light" | "dark"),
            "language" => matches!(value, "system" | "ja" | "en"),
            "toolbarDensity" => matches!(value, "compact" | "comfortable"),
            "sidebarVisible" => matches!(value, "show" | "hide"),
            "defaultBookmarkDisplay" => matches!(value, "sidebar" | "page"),
            "openBookmarkIn" => matches!(value, "current" | "newTab"),
            "showBookmarkFavicons" | "askBeforeDownload" | "closeWindowWithLastTab" => {
                matches!(value, "on" | "off")
            }
            "newTabPage" => matches!(value, "blank" | "home"),
            "privateSearchEngine" => matches!(value, "default" | "duckduckgo"),
            "newTabBackgroundMode" => matches!(value, "unsplash" | "color" | "custom"),
            "sidebarWidth" => value
                .parse::<u32>()
                .is_ok_and(|width| (160..=480).contains(&width)),
            "defaultZoomLevel" => value
                .parse::<f64>()
                .is_ok_and(|zoom| (-5.0..=5.0).contains(&zoom)),
            "customSearchUrl" => value.contains("{query}") && Self::safe_url(value),
            "homepage" | "homeUrl" => Self::safe_url(value),
            "newTabBackgroundUrl" => value.is_empty() || Self::safe_url(value),
            "newTabBackgroundColor" => {
                value.len() == 7
                    && value.starts_with('#')
                    && value[1..]
                        .chars()
                        .all(|character| character.is_ascii_hexdigit())
            }
            "downloadDirectory" => value.is_empty() || std::path::Path::new(value).is_absolute(),
            _ => !value.contains('\0'),
        }
    }

    fn safe_url(value: &str) -> bool {
        value.starts_with("https://")
            || value.starts_with("http://")
            || value.starts_with("fubuki://")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Settings(std::cell::RefCell<std::collections::HashMap<String, String>>);

    impl SettingsRepository for Settings {
        fn get_setting(&self, key: &str) -> StoreResult<Option<String>> {
            Ok(self.0.borrow().get(key).cloned())
        }

        fn set_setting(&self, key: &str, value: &str) -> StoreResult<()> {
            self.0.borrow_mut().insert(key.into(), value.into());
            Ok(())
        }
    }

    #[test]
    fn validates_settings_at_the_engine_boundary() {
        let settings = Settings::default();
        assert!(SettingsService::set(&settings, "sidebarWidth", "240").is_ok());
        assert!(SettingsService::set(&settings, "sidebarWidth", "9999").is_err());
        assert!(SettingsService::set(&settings, "downloadDirectory", "relative").is_err());
        assert!(SettingsService::set(&settings, "customSearchUrl", "javascript:{query}").is_err());
        assert!(SettingsService::set(&settings, "unknown", "value").is_err());
    }
}
