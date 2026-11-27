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
    pathRef,
    listingReady,
    entries,
    setPath,
    setSelection,
    setListEpoch,
    setCarryReady,
    setCarryNote,
    setCarryError,
    setCarryRequested,
    setLoadError,
    carryGen,
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

  useEffect(() => {
    if (!carryReady) return;
    let discarded = false;
    const requestedPath = path;
    const requestedIdentity = activeIdentity;
    const cached = peekListDir(repoName, snapshotId, requestedPath);
    if (cached) {
      setEntries(cached);
      setLoadedPath(requestedPath);
      setLoadedIdentity(requestedIdentity);
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
