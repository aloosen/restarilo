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
  const [loadedListing, setLoadedListing] = useState<{
    path: string; identity: string; epoch: number;
  } | null>(null);
  const prevRepo = useRef(repoName);
  const prevSnap = useRef(snapshotId);
  const pathRef = useRef(path);
  const carryGen = useRef(0);
  pathRef.current = path;

  const listingCurrent =
    loadedListing?.path === path && loadedListing.identity === activeIdentity &&
    loadedListing.epoch === listEpoch;
  const listingReady =
    carryReady && !loading && listingCurrent && !loadError && !carryError;

  usePendingShowHit({
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
      setPath("/");
      setCarryNote(null); setCarryError(null); setCarryRequested(null);
      setSelection(null); setEntries([]); setLoadError(null); setActionError(null);
      setLoading(true); setLoadedListing(null); setCarryReady(true);
      setListEpoch((n) => n + 1);
      return;
    }
    const snapshotChanged = prevSnap.current !== snapshotId;
    prevSnap.current = snapshotId;
    if (showHit?.snapshotId === snapshotId) {
      const parent = parentDirOf(showHit.path);
      if (snapshotChanged || pathRef.current !== parent || !carryReady) navigateTo(parent);
      return;
    }
    if (snapshotChanged) beginCarry(pathRef.current);
  }, [repoName, snapshotId, showHit]);

  useEffect(() => {
    if (!carryReady) return;
    let discarded = false;
    const requestedPath = path;
    const requestedListing = { path, identity: activeIdentity, epoch: listEpoch };
    const cached = peekListDir(repoName, snapshotId, requestedPath);
    if (cached) {
      setEntries(cached);
      setLoadedListing(requestedListing);
      setLoadError(null);
      setSelection(null);
      setLoading(false);
      return;
    }
    setLoading(true);
    setSelection(null);
    setLoadError(null);
    loadListDir(repoName, snapshotId, requestedPath)
      .then((rows) => {
        if (discarded) return;
        setEntries(rows);
        setLoadedListing(requestedListing);
      })
      .catch((e: unknown) => {
        if (discarded) return;
        setEntries([]);
        setLoadError(String(e));
        setLoadedListing(requestedListing);
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
    showStaleListing, setSelection,
    navigateTo: (next: string) => {
      if (showHit) onShowHitConsumed?.();
      navigateTo(next);
    },
    retryListing: () => navigateTo(path),
    retryCarry: () => beginCarry(carryRequested ?? path),
  };
}
