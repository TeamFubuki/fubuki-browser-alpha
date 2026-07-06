use frost_store::{SettingsRepository, StoreResult};

pub struct SettingsService;

impl SettingsService {
    pub fn get<S: SettingsRepository>(repository: &S, key: &str) -> StoreResult<Option<String>> {
        repository.get_setting(key)
    }

    pub fn set<S: SettingsRepository>(repository: &S, key: &str, value: &str) -> StoreResult<()> {
        repository.set_setting(key, value)
    }
}
