use frost_store::{SettingsRepository, StoreResult};

pub struct SettingsService<S> {
    repository: S,
}

impl<S> SettingsService<S>
where
    S: SettingsRepository,
{
    pub fn new(repository: S) -> Self {
        Self { repository }
    }

    pub fn get(&self, key: &str) -> StoreResult<Option<String>> {
        self.repository.get_setting(key)
    }

    pub fn set(&self, key: &str, value: &str) -> StoreResult<()> {
        self.repository.set_setting(key, value)
    }
}
