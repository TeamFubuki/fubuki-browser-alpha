use frost_protocol::DownloadRecord;
use frost_store::{DownloadRepository, StoreResult};

pub struct DownloadService;

impl DownloadService {
    pub fn list<S: DownloadRepository>(repository: &S) -> StoreResult<Vec<DownloadRecord>> {
        repository.list_downloads()
    }

    pub fn remove<S: DownloadRepository>(
        repository: &S,
        url: Option<&str>,
        path: Option<&str>,
    ) -> StoreResult<bool> {
        repository.remove_download(url, path)
    }
}
