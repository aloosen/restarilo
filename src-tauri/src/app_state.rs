use crate::tree_cache::TreeCache;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub tree_cache: Arc<TreeCache>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tree_cache: Arc::new(TreeCache::new()),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
