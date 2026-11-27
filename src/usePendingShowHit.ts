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

/** Tracks a pending show-hit and applies selection when listing is ready. */
export function usePendingShowHit(opts: {
  showHit: ShowHitRequest | null;
  snapshotId: string;
  path: string;
  pathRef: { current: string };
  listingReady: boolean;
  entries: DirEntry[];
  setPath: Dispatch<SetStateAction<string>>;
  setSelection: Dispatch<SetStateAction<FileSelection | null>>;
  setListEpoch: Dispatch<SetStateAction<number>>;
  setCarryReady: Dispatch<SetStateAction<boolean>>;
  setCarryNote: Dispatch<SetStateAction<string | null>>;
  setCarryError: Dispatch<SetStateAction<string | null>>;
  setCarryRequested: Dispatch<SetStateAction<string | null>>;
  setLoadError: Dispatch<SetStateAction<string | null>>;
  carryGen: { current: number };
  onShowHitConsumed?: () => void;
}): { current: ShowHitRequest | null } {
  const pendingShow = useRef<ShowHitRequest | null>(null);
  const showHitRef = useRef(opts.showHit);
  showHitRef.current = opts.showHit;

  useEffect(() => {
    if (!opts.showHit || opts.showHit.snapshotId !== opts.snapshotId) return;
    pendingShow.current = opts.showHit;
    const parent = parentDirOf(opts.showHit.path);
    if (opts.pathRef.current !== parent) {
      opts.carryGen.current += 1;
      opts.setCarryReady(true);
      opts.setCarryNote(null);
      opts.setCarryError(null);
      opts.setCarryRequested(null);
      opts.setLoadError(null);
      opts.setPath(parent);
    } else opts.setListEpoch((n) => n + 1);
  }, [opts.showHit, opts.snapshotId]);

  useEffect(() => {
    const pending = pendingShow.current;
    if (!pending || !opts.listingReady) return;
    if (pending.snapshotId !== opts.snapshotId) return;
    if (opts.path !== parentDirOf(pending.path)) return;
    const entry = opts.entries.find(
      (e) => e.path === pending.path || e.name === pending.name,
    );
    if (entry) {
      opts.setSelection({
        path: entry.path,
        name: entry.name,
        is_dir: entry.is_dir,
      });
    }
    pendingShow.current = null;
    opts.onShowHitConsumed?.();
  }, [
    opts.listingReady,
    opts.entries,
    opts.path,
    opts.snapshotId,
    opts.onShowHitConsumed,
  ]);

  return pendingShow;
}

export function showHitRefCurrent(
  showHitRef: { current: ShowHitRequest | null },
): ShowHitRequest | null {
  return showHitRef.current;
}
