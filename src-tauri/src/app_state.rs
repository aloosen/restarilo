use crate::folder_index::FolderIndex;
use crate::scan_scheduler::ScanScheduler;
use crate::tree_cache::TreeCache;
use crate::tree_prefetch::PrefetchQueue;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub tree_cache: Arc<TreeCache>,
    pub prefetch: Arc<PrefetchQueue>,
    pub folder_index: Arc<FolderIndex>,
    pub scan: Arc<ScanScheduler>,
}

impl AppState {
    pub fn new() -> Self {
        let tree_cache = Arc::new(TreeCache::new());
        let prefetch = PrefetchQueue::new(tree_cache.clone());
        let folder_index = Arc::new(FolderIndex::new());
        let scan = ScanScheduler::new(folder_index.clone());
        Self {
            tree_cache,
            prefetch,
            folder_index,
            scan,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
