use frost_protocol::HistoryRecord;
use frost_store::{HistoryRepository, StoreResult};

pub struct HistoryService;

impl HistoryService {
    pub fn list<S: HistoryRepository>(repository: &S) -> StoreResult<Vec<HistoryRecord>> {
        repository.list_history()
    }

    pub fn remove<S: HistoryRepository>(repository: &S, url: &str) -> StoreResult<bool> {
        repository.remove_history(url)
    }

    pub fn clear_range<S: HistoryRepository>(repository: &S, range: &str) -> StoreResult<bool> {
        repository.clear_history_range(range)
    }
}
