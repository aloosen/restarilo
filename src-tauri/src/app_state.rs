use crate::tree_cache::TreeCache;
use crate::tree_prefetch::PrefetchQueue;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub tree_cache: Arc<TreeCache>,
    pub prefetch: Arc<PrefetchQueue>,
}

impl AppState {
    pub fn new() -> Self {
        let tree_cache = Arc::new(TreeCache::new());
        let prefetch = PrefetchQueue::new(tree_cache.clone());
        Self {
            tree_cache,
            prefetch,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
