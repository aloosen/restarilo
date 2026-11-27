mod app_state;
mod commands;
mod commands_folder;
mod commands_smb;
mod config;
mod find_pattern;
mod folder_debug;
mod folder_index;
mod folder_index_emit;
mod restic_actions;
mod restic_bin;
mod restic_cmd;
mod restic_find;
mod restic_killable;
mod restic_tag;
mod restic_ls;
mod restic_tree;
mod scan_scheduler;
mod smb_probe;
mod smb_signin;
mod smb_unc;
mod ssh;
mod subtree_scan;
mod subtree_scan_run;
mod tree_cache;
mod tree_disk_cache;
mod tree_prefetch;
mod tree_walk;

#[cfg(test)]
mod z1_acceptance_bench;
#[cfg(test)]
mod z1_click_bench;
#[cfg(test)]
mod z1_click_bench_after;

use app_state::AppState;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;
use tauri::{Emitter, Manager};

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
    {
        let scan = state.scan.clone();
        state.scan.set_runner(Some(Arc::new(move |job| {
            let Ok(cfg) = commands::cached_config() else {
                return subtree_scan::ScanStop::Failed;
            };
            let Ok(mut cmd) = restic_cmd::prepare_restic_command(cfg, &job.repo) else {
                return subtree_scan::ScanStop::Failed;
            };
            cmd.args([
                "ls",
                &job.snapshot_id,
                &job.root,
                "--recursive",
                "--json",
            ]);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::null());
            cmd.stdin(Stdio::null());
            let Ok(child) = cmd.spawn() else {
                return subtree_scan::ScanStop::Failed;
            };
            let gen = job.generation;
            let repo = job.repo.clone();
            let snap = job.snapshot_id.clone();
            let root = job.root.clone();
            let scan_ref = scan.clone();
            let repo_c = repo.clone();
            let snap_c = snap.clone();
            subtree_scan_run::run_scan_command(
                child,
                &root,
                gen,
                &repo,
                &snap,
                &job.index,
                subtree_scan_run::SpawnOpts {
                    wall_time: Duration::from_secs(15),
                    node_budget: subtree_scan::MAX_DIR_NODES,
                    cancel: Some(job.cancel.clone()),
                },
                move |parent, dirs| {
                    scan_ref.emit_dirs(gen, &repo_c, &snap_c, parent, dirs, false);
                },
            )
        })));
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state)
        .setup(|app| {
            let handle = app.handle().clone();
            let scan = app.state::<AppState>().scan.clone();
            scan.set_webview_emit(Some(Arc::new(move |ev| {
                let _ = handle.emit("folder-index-updated", &ev);
            })));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::restic_version,
            commands::list_repos,
            commands_folder::list_snapshots,
            commands_folder::list_dir,
            commands_folder::list_folder_dirs,
            commands_smb::probe_share_access,
            commands_smb::sign_in_network_share,
            commands::begin_action,
            commands::cancel_action,
            commands::restore_path,
            commands::dump_path,
            commands::open_path,
            commands::find_paths,
            commands::set_snapshot_tags,
            folder_debug::folder_debug_path,
            folder_debug::folder_debug_fe
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
