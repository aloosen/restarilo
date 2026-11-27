import { useEffect, type Dispatch, type SetStateAction } from "react";
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
): void {
  const entry = entries.find((e) => e.path === pending.path);
  if (!entry) return;
  setSelection({
    path: entry.path,
    name: entry.name,
    is_dir: entry.is_dir,
  });
}

export function usePendingShowHit(opts: {
  showHit: ShowHitRequest | null;
  snapshotId: string;
  path: string;
  listingReady: boolean;
  entries: DirEntry[];
  setSelection: Dispatch<SetStateAction<FileSelection | null>>;
  onShowHitConsumed?: () => void;
}): void {
  useEffect(() => {
    const pending = opts.showHit;
    if (!pending || !opts.listingReady) return;
    if (pending.snapshotId !== opts.snapshotId) return;
    if (opts.path !== parentDirOf(pending.path)) return;
    selectHit(opts.entries, pending, opts.setSelection);
    opts.onShowHitConsumed?.();
  }, [
    opts.showHit,
    opts.listingReady,
    opts.entries,
    opts.path,
    opts.snapshotId,
    opts.setSelection,
    opts.onShowHitConsumed,
  ]);
}
