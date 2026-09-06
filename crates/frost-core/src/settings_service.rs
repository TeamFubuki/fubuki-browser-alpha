use frost_store::{SettingsRepository, StoreError, StoreResult, VALID_SETTING_KEYS};

pub struct SettingsService;

impl SettingsService {
    /// Valid setting keys that can be stored and retrieved.
    pub const VALID_KEYS: &[&str] = VALID_SETTING_KEYS;

    /// Returns the default value for a given setting key.
    pub fn default_value(key: &str) -> &'static str {
        match key {
            "homepage" | "homeUrl" => "https://example.com",
            "downloadDirectory" => "",
            "searchEngine" => "google",
            "startupBehavior" => "lastSession",
            "customSearchUrl" => "https://www.google.com/search?q={query}",
            "theme" => "light",
            "appearance" => "system",
            "toolbarDensity" => "compact",
            "sidebarVisible" => "show",
            "sidebarWidth" => "196",
            "defaultBookmarkDisplay" => "sidebar",
            "openBookmarkIn" => "current",
            "showBookmarkFavicons" => "true",
            "newTabPage" => "blank",
            "askBeforeDownload" => "false",
            "language" => "system",
            "defaultZoomLevel" => "0",
            "closeWindowWithLastTab" => "false",
            "privateSearchEngine" => "default",
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
        repository.set_setting(key, value)
    }
}
