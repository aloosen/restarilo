mod app_state;
mod commands;
mod config;
mod folder_index;
mod restic_actions;
mod restic_bin;
mod restic_cmd;
mod restic_killable;
mod restic_ls;
mod restic_tree;
mod ssh;
mod tree_cache;
mod tree_disk_cache;
mod subtree_scan;
mod subtree_scan_run;
mod tree_prefetch;
mod tree_walk;

#[cfg(test)]
mod z1_acceptance_bench;

use app_state::AppState;
use std::sync::Arc;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState::new();
    state.tree_cache.set_fetcher(|repo, tree_id| {
        let cfg = commands::cached_config()?;
        restic_tree::cat_tree(cfg, repo, tree_id)
    });
    if let Ok(cfg) = commands::cached_config() {
        if let Ok(Some(dir)) = config::resolve_cache_dir(cfg) {
            state
                .tree_cache
                .set_disk(Some(Arc::new(tree_disk_cache::TreeDiskCache::open(dir))));
        }
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            commands::restic_version,
            commands::list_repos,
            commands::list_snapshots,
            commands::list_dir,
            commands::begin_action,
            commands::cancel_action,
            commands::restore_path,
            commands::dump_path,
            commands::open_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
