use frost_store::{SettingsRepository, StoreError, StoreResult};

pub struct SettingsService;

impl SettingsService {
    /// Valid setting keys that can be stored and retrieved.
    pub const VALID_KEYS: &[&str] = &[
        "homepage",
        "startupBehavior",
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
        "askBeforeDownload",
    ];

    /// Returns the default value for a given setting key.
    pub fn default_value(key: &str) -> &'static str {
        match key {
            "homepage" | "homeUrl" => "https://example.com",
            "startupBehavior" => "newTab",
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
            "askBeforeDownload" => "off",
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
        Self::validate(key, value)?;
        repository.set_setting(key, value)
    }

    pub fn validate(key: &str, value: &str) -> StoreResult<()> {
        if !Self::VALID_KEYS.contains(&key) {
            return Err(StoreError::InvalidKey(key.to_owned()));
        }
        let valid = match key {
            "startupBehavior" => matches!(value, "restore" | "newTab" | "homePage"),
            "appearance" => matches!(value, "system" | "light" | "dark"),
            "theme" => matches!(value, "light" | "dark" | "system"),
            "sidebarVisible" => matches!(value, "show" | "hide"),
            "sidebarWidth" => value
                .parse::<u16>()
                .is_ok_and(|width| (168..=400).contains(&width)),
            "defaultZoomLevel" => value
                .parse::<f64>()
                .is_ok_and(|zoom| (-5.0..=5.0).contains(&zoom)),
            "askBeforeDownload" => matches!(value, "on" | "off"),
            _ => !value.contains('\0'),
        };
        if valid {
            Ok(())
        } else {
            Err(StoreError::InvalidValue {
                key: key.into(),
                value: value.into(),
            })
        }
    }
}
