import { invoke } from "@tauri-apps/api/core";

export type DirEntry = {
  name: string;
  path: string;
  is_dir: boolean;
  entry_type: string;
  size?: number | null;
  mtime?: string | null;
};

export type CarryResult =
  | { ok: true; path: string; note: string | null }
  | { ok: false; path: string; error: string; requested: string };

type CacheKey = string;

const success = new Map<CacheKey, DirEntry[]>();
const inflight = new Map<CacheKey, Promise<DirEntry[]>>();
const repoEpoch = new Map<string, number>();

function key(repoName: string, snapshotId: string, path: string): CacheKey {
  return `${repoName}\0${snapshotId}\0${path}`;
}

function epoch(repoName: string): number {
  return repoEpoch.get(repoName) ?? 0;
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

/** Drop cached + in-flight entries for a repo; bump epoch so late fills cannot re-seed. */
export function clearListDirCacheForRepo(repoName: string): void {
  repoEpoch.set(repoName, epoch(repoName) + 1);
  const prefix = `${repoName}\0`;
  for (const k of [...success.keys()]) {
    if (k.startsWith(prefix)) success.delete(k);
  }
  for (const k of [...inflight.keys()]) {
    if (k.startsWith(prefix)) inflight.delete(k);
  }
}

export function invalidateListDir(
  repoName: string,
  snapshotId: string,
  path: string,
): void {
  const k = key(repoName, snapshotId, path);
  success.delete(k);
  // Drop in-flight ownership so a late finally cannot touch a newer promise,
  // and a retry starts a fresh request instead of joining a doomed one.
  inflight.delete(k);
}

export function loadListDir(
  repoName: string,
  snapshotId: string,
  path: string,
): Promise<DirEntry[]> {
  const k = key(repoName, snapshotId, path);
  const startedEpoch = epoch(repoName);
  const hit = success.get(k);
  if (hit) return Promise.resolve(hit);

  const pending = inflight.get(k);
  if (pending) return pending;

  let req: Promise<DirEntry[]>;
  req = invoke<DirEntry[]>("list_dir", { repoName, snapshotId, path })
    .then((rows) => {
      // Only seed cache if this promise is still the owner for `k`.
      if (epoch(repoName) === startedEpoch && inflight.get(k) === req) {
        success.set(k, rows);
      }
      return rows;
    })
    .finally(() => {
      // Do not clear a successor that replaced us after cache clear / invalidate.
      if (inflight.get(k) === req) inflight.delete(k);
    });

  inflight.set(k, req);
  return req;
}

/**
 * Keep `requested` when it exists as a directory (incl. empty).
 * Missing segment → ancestor fallback with note.
 * Failed parent lookup → ok:false (caller shows retry for `requested`).
 */
export async function resolveCarryPath(
  repoName: string,
  snapshotId: string,
  requested: string,
): Promise<CarryResult> {
  if (requested === "/" || requested === "") return { ok: true, path: "/", note: null };
  const segs = requested.split("/").filter(Boolean);
  let acc = "";
  let lastDir = "/";
  for (const seg of segs) {
    const parent = acc === "" ? "/" : acc;
    let rows: DirEntry[];
    try {
      rows = await loadListDir(repoName, snapshotId, parent);
    } catch (e: unknown) {
      return {
        ok: false,
        path: lastDir,
        error: String(e),
        requested,
      };
    }
    const child = rows.find((e) => e.name === seg);
    if (!child || !child.is_dir) {
      const note =
        lastDir === requested ? null : `Requested ${requested}; showing ${lastDir}`;
      return { ok: true, path: lastDir, note };
    }
    acc = `${acc}/${seg}`;
    lastDir = acc;
  }
  return { ok: true, path: lastDir, note: null };
}
