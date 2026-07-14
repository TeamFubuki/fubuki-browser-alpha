use frost_store::{SettingsRepository, StoreError, StoreResult};

pub struct SettingsService;

impl SettingsService {
    /// Valid setting keys that can be stored and retrieved.
    pub const VALID_KEYS: &[&str] = &[
        "homepage",
        "searchEngine",
        "customSearchUrl",
        "theme",
        "appearance",
        "sidebarVisible",
        "sidebarWidth",
        "newTabPage",
        "homeUrl",
        "language",
        "defaultZoomLevel",
        "downloadDirectory",
        "startupBehavior",
        "newTabBackgroundMode",
        "newTabBackgroundColor",
        "newTabBackgroundUrl",
        "toolbarDensity",
        "defaultBookmarkDisplay",
        "openBookmarkIn",
        "showBookmarkFavicons",
        "askBeforeDownload",
        "closeWindowWithLastTab",
        "privateSearchEngine",
    ];

    /// Returns the default value for a given setting key.
    pub fn default_value(key: &str) -> &'static str {
        match key {
            "homepage" | "homeUrl" => "https://example.com",
            "searchEngine" => "google",
            "customSearchUrl" => "https://www.google.com/search?q={query}",
            "theme" => "light",
            "appearance" => "system",
            "sidebarVisible" => "show",
            "sidebarWidth" => "196",
            "newTabPage" => "blank",
            "language" => "system",
            "defaultZoomLevel" => "0",
            "downloadDirectory" => "",
            "startupBehavior" => "newTab",
            "newTabBackgroundMode" => "unsplash",
            "newTabBackgroundColor" => "#f8fafd",
            "newTabBackgroundUrl" => "",
            "toolbarDensity" => "compact",
            "defaultBookmarkDisplay" => "sidebar",
            "openBookmarkIn" => "current",
            "showBookmarkFavicons" => "on",
            "askBeforeDownload" => "off",
            "closeWindowWithLastTab" => "off",
            "privateSearchEngine" => "default",
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
