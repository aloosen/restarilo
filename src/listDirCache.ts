import { invoke } from "@tauri-apps/api/core";

export type DirEntry = {
  name: string;
  path: string;
  is_dir: boolean;
  size?: number | null;
};

type CacheKey = string;

const success = new Map<CacheKey, DirEntry[]>();
const inflight = new Map<CacheKey, Promise<DirEntry[]>>();

function key(repoName: string, snapshotId: string, path: string): CacheKey {
  return `${repoName}\0${snapshotId}\0${path}`;
}

export function parentPath(path: string): string {
  if (path === "/" || path === "") return "/";
  let end = path.length;
  while (end > 1 && path.charAt(end - 1) === "/") end -= 1;
  const trimmed = path.slice(0, end);
  const idx = trimmed.lastIndexOf("/");
  if (idx <= 0) return "/";
  return trimmed.slice(0, idx);
}

export function clearListDirCacheForRepo(repoName: string): void {
  const prefix = `${repoName}\0`;
  for (const k of [...success.keys()]) {
    if (k.startsWith(prefix)) success.delete(k);
  }
  for (const k of [...inflight.keys()]) {
    if (k.startsWith(prefix)) inflight.delete(k);
  }
}

export function loadListDir(
  repoName: string,
  snapshotId: string,
  path: string,
): Promise<DirEntry[]> {
  const k = key(repoName, snapshotId, path);
  const hit = success.get(k);
  if (hit) return Promise.resolve(hit);

  const pending = inflight.get(k);
  if (pending) return pending;

  const req = invoke<DirEntry[]>("list_dir", { repoName, snapshotId, path })
    .then((rows) => {
      success.set(k, rows);
      return rows;
    })
    .finally(() => {
      inflight.delete(k);
    });

  inflight.set(k, req);
  return req;
}
