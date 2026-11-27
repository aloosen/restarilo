import { useEffect, useRef, useState } from "react";
import type { FileSelection } from "./FileActions";
import {
  clearListDirCacheForRepo,
  loadListDir,
  peekListDir,
  resolveCarryPath,
  type CarryResult,
  type DirEntry,
} from "./listDirCache";
import {
  parentDirOf,
  usePendingShowHit,
  type ShowHitRequest,
} from "./usePendingShowHit";

export type { ShowHitRequest };
export { parentDirOf };

export function useExplorerBrowse(
  repoName: string,
  snapshotId: string,
  showHit: ShowHitRequest | null = null,
  onShowHitConsumed?: () => void,
) {
  const activeIdentity = `${repoName}\0${snapshotId}`;
  const [path, setPath] = useState("/");
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [selection, setSelection] = useState<FileSelection | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [carryNote, setCarryNote] = useState<string | null>(null);
  const [carryError, setCarryError] = useState<string | null>(null);
  const [carryRequested, setCarryRequested] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [carryReady, setCarryReady] = useState(true);
  const [listEpoch, setListEpoch] = useState(0);
  const [loadedPath, setLoadedPath] = useState<string | null>(null);
  const [loadedIdentity, setLoadedIdentity] = useState<string | null>(null);
  const prevRepo = useRef(repoName);
  const prevSnap = useRef(snapshotId);
  const pathRef = useRef(path);
  const carryGen = useRef(0);
  const showHitRef = useRef(showHit);
  showHitRef.current = showHit;
  pathRef.current = path;

  const listingCurrent =
    loadedPath === path && loadedIdentity === activeIdentity;
  const listingReady =
    carryReady && !loading && listingCurrent && !loadError && !carryError;

  const pendingShow = usePendingShowHit({
    showHit,
    snapshotId,
    path,
    listingReady,
    entries,
    setSelection,
    onShowHitConsumed,
  });

  function applyCarry(gen: number, r: CarryResult) {
    if (gen !== carryGen.current) return;
    setPath(r.path);
    if (!r.ok) {
      setCarryError(r.error);
      setCarryRequested(r.requested);
      setCarryNote(null);
    } else {
      setCarryNote(r.note);
      setCarryError(null);
      setCarryRequested(null);
    }
    setCarryReady(true);
    setListEpoch((n) => n + 1);
  }

  function beginCarry(requested: string) {
    const gen = ++carryGen.current;
    setSelection(null);
    setActionError(null);
    setCarryNote(null);
    setCarryError(null);
    setCarryRequested(null);
    setCarryReady(false);
    setLoading(true);
    void resolveCarryPath(repoName, snapshotId, requested).then((r) =>
      applyCarry(gen, r),
    );
  }

  function navigateTo(next: string) {
    carryGen.current += 1;
    setCarryReady(true);
    setCarryNote(null);
    setCarryError(null);
    setCarryRequested(null);
    setLoadError(null);
    if (next === pathRef.current) setListEpoch((n) => n + 1);
    else setPath(next);
  }

  useEffect(() => {
    if (prevRepo.current !== repoName) {
      clearListDirCacheForRepo(prevRepo.current);
      prevRepo.current = repoName;
      prevSnap.current = snapshotId;
      carryGen.current += 1;
      pendingShow.current = null;
      setPath("/");
      setCarryNote(null); setCarryError(null); setCarryRequested(null);
      setSelection(null); setEntries([]); setLoadError(null); setActionError(null);
      setLoading(true); setLoadedPath(null); setLoadedIdentity(null); setCarryReady(true);
      setListEpoch((n) => n + 1);
      return;
    }
    if (prevSnap.current === snapshotId) return;
    prevSnap.current = snapshotId;
    const hit = showHitRef.current;
    if (hit && hit.snapshotId === snapshotId) {
      pendingShow.current = hit;
      beginCarry(parentDirOf(hit.path));
      return;
    }
    pendingShow.current = null;
    beginCarry(pathRef.current);
  }, [repoName, snapshotId]);

  // Same-snapshot Show: open parent. Skip while a snapshot carry is in flight
  // so we do not cancel cross-snapshot beginCarry(parent).
  useEffect(() => {
    if (!showHit || showHit.snapshotId !== snapshotId || !carryReady) return;
    const parent = parentDirOf(showHit.path);
    if (pathRef.current === parent) return;
    navigateTo(parent);
  }, [showHit, snapshotId, carryReady]);

  useEffect(() => {
    if (!carryReady) return;
    let discarded = false;
    const requestedPath = path;
    const requestedIdentity = activeIdentity;
    const cached = peekListDir(repoName, snapshotId, requestedPath);
    const keepSel = pendingShow.current !== null;
    if (cached) {
      setEntries(cached);
      setLoadedPath(requestedPath);
      setLoadedIdentity(requestedIdentity);
      setLoadError(null);
      if (!keepSel) setSelection(null);
      setLoading(false);
      return;
    }
    setLoading(true);
    if (!keepSel) setSelection(null);
    setLoadError(null);
    loadListDir(repoName, snapshotId, requestedPath)
      .then((rows) => {
        if (discarded) return;
        setEntries(rows);
        setLoadedPath(requestedPath);
        setLoadedIdentity(requestedIdentity);
      })
      .catch((e: unknown) => {
        if (discarded) return;
        setEntries([]);
        setLoadError(String(e));
        setLoadedPath(requestedPath);
        setLoadedIdentity(requestedIdentity);
      })
      .finally(() => {
        if (!discarded) setLoading(false);
      });
    return () => {
      discarded = true;
    };
  }, [repoName, snapshotId, path, activeIdentity, carryReady, listEpoch]);

  const showStaleListing =
    !listingReady &&
    entries.length > 0 &&
    !loadError &&
    !carryError &&
    (loading || !carryReady);

  return {
    path, entries, selection, loadError, actionError, setActionError,
    carryNote, carryError, carryRequested, loading, carryReady, listingReady,
    showStaleListing, navigateTo, setSelection,
    retryListing: () => navigateTo(path),
    retryCarry: () => beginCarry(carryRequested ?? path),
  };
}
