import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { DirEntry } from "./listDirCache";

export type FolderDirsResult = {
  dirs: { name: string; path: string }[];
  children_complete: boolean;
  scan_status: string;
};

export type FolderIndexUpdated = {
  repoName: string;
  snapshotId: string;
  generation: number;
  parentPath: string;
  dirs: { name: string; path: string }[];
  childrenComplete: boolean;
  scanStatus: string;
};

function toDirEntries(
  dirs: { name: string; path: string }[],
): DirEntry[] {
  return dirs.map((d) => ({
    name: d.name,
    path: d.path,
    is_dir: true,
    entry_type: "dir",
  }));
}

/** Normalize serde snake_case / camelCase from Rust. */
function normalizeEvent(raw: Record<string, unknown>): FolderIndexUpdated | null {
  const repoName = (raw.repoName ?? raw.repo_name) as string | undefined;
  const snapshotId = (raw.snapshotId ?? raw.snapshot_id) as string | undefined;
  const parentPath = (raw.parentPath ?? raw.parent_path) as string | undefined;
  if (!repoName || !snapshotId || parentPath === undefined) return null;
  const dirs = (raw.dirs as { name: string; path: string }[]) ?? [];
  return {
    repoName,
    snapshotId,
    generation: Number(raw.generation ?? 0),
    parentPath,
    dirs,
    childrenComplete: Boolean(raw.childrenComplete ?? raw.children_complete),
    scanStatus: String(raw.scanStatus ?? raw.scan_status ?? ""),
  };
}

export function loadFolderDirs(
  repoName: string,
  snapshotId: string,
  path: string,
  prioritize: boolean,
): Promise<FolderDirsResult> {
  return invoke<FolderDirsResult>("list_folder_dirs", {
    repoName,
    snapshotId,
    path,
    prioritize,
  });
}

export function subscribeFolderIndex(
  repoName: string,
  snapshotId: string,
  onUpdate: (ev: FolderIndexUpdated) => void,
): Promise<UnlistenFn> {
  return listen<Record<string, unknown>>("folder-index-updated", (e) => {
    const ev = normalizeEvent(e.payload);
    if (!ev) return;
    if (ev.repoName !== repoName || ev.snapshotId !== snapshotId) return;
    onUpdate(ev);
  });
}

export function mergeFolderDirs(
  prev: DirEntry[],
  incoming: { name: string; path: string }[],
): DirEntry[] {
  const map = new Map(prev.map((d) => [d.name, d]));
  for (const d of toDirEntries(incoming)) map.set(d.name, d);
  return [...map.values()].sort((a, b) => a.name.localeCompare(b.name));
}

export { toDirEntries };
