import { useEffect, useRef, type Dispatch, type SetStateAction } from "react";
import type { FileSelection } from "./FileActions";
import type { DirEntry } from "./listDirCache";

export type ShowHitRequest = {
  snapshotId: string;
  path: string;
  name: string;
};

export function parentDirOf(path: string): string {
  const trimmed = path.replace(/\/+$/, "");
  if (!trimmed || trimmed === "/") return "/";
  const i = trimmed.lastIndexOf("/");
  return i <= 0 ? "/" : trimmed.slice(0, i);
}

function selectHit(
  entries: DirEntry[],
  pending: ShowHitRequest,
  setSelection: Dispatch<SetStateAction<FileSelection | null>>,
): boolean {
  const entry = entries.find(
    (e) => e.path === pending.path || e.name === pending.name,
  );
  if (!entry) return false;
  setSelection({
    path: entry.path,
    name: entry.name,
    is_dir: entry.is_dir,
  });
  return true;
}

/**
 * Holds a show-hit until listingReady for that snapshot+parent, then selects.
 * Navigation is owned by useExplorerBrowse (same-snap navigate / cross-snap carry).
 */
export function usePendingShowHit(opts: {
  showHit: ShowHitRequest | null;
  snapshotId: string;
  path: string;
  listingReady: boolean;
  entries: DirEntry[];
  setSelection: Dispatch<SetStateAction<FileSelection | null>>;
  onShowHitConsumed?: () => void;
}): { current: ShowHitRequest | null } {
  const pendingShow = useRef<ShowHitRequest | null>(null);
  const consumedRef = useRef(opts.onShowHitConsumed);
  consumedRef.current = opts.onShowHitConsumed;

  useEffect(() => {
    if (!opts.showHit || opts.showHit.snapshotId !== opts.snapshotId) return;
    pendingShow.current = opts.showHit;
  }, [opts.showHit, opts.snapshotId]);

  useEffect(() => {
    const pending = pendingShow.current;
    if (!pending || !opts.listingReady) return;
    if (pending.snapshotId !== opts.snapshotId) return;
    if (opts.path !== parentDirOf(pending.path)) return;
    if (!selectHit(opts.entries, pending, opts.setSelection)) return;
    pendingShow.current = null;
    consumedRef.current?.();
  }, [
    opts.listingReady,
    opts.entries,
    opts.path,
    opts.snapshotId,
    opts.setSelection,
  ]);

  return pendingShow;
}
