use frost_protocol::BookmarkRecord;
use frost_store::{BookmarkRepository, StoreResult};

pub struct BookmarkService;

impl BookmarkService {
    pub fn list<S: BookmarkRepository>(repository: &S) -> StoreResult<Vec<BookmarkRecord>> {
        repository.list_bookmarks()
    }

    pub fn save<S: BookmarkRepository>(
        repository: &S,
        title: &str,
        url: &str,
        favicon_url: &str,
    ) -> StoreResult<bool> {
        repository.save_bookmark(title, url, favicon_url)
    }

    pub fn remove<S: BookmarkRepository>(repository: &S, url: &str) -> StoreResult<bool> {
        repository.remove_bookmark(url)
    }
}
