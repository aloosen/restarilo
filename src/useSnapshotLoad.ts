import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { clearListDirCacheForRepo } from "./listDirCache";
import {
  isOk,
  isShareTimeoutError,
  shareTimeoutStatus,
  type ShareAccessStatus,
} from "./shareAccess";
import {
  sortSnapshotsNewestFirst,
  type SnapshotRow,
} from "./SnapshotList";

/** Exported for D2 token-gate harness / unit-style checks. */
export function shouldInvokeListSnapshots(
  tokenAtStart: number,
  currentToken: number,
  probe: ShareAccessStatus | null,
): boolean {
  return tokenAtStart === currentToken && isOk(probe);
}

async function loadForRepo(
  repoName: string,
  token: number,
  tokenRef: { current: number },
): Promise<{
  snapshots: SnapshotRow[];
  selected: SnapshotRow | null;
  snapError: string | null;
  shareStatus: ShareAccessStatus | null;
}> {
  const probe = await invoke<ShareAccessStatus>("probe_share_access", {
    repoName,
  });
  const empty = {
    snapshots: [] as SnapshotRow[],
    selected: null as SnapshotRow | null,
    snapError: null as string | null,
    shareStatus: null as ShareAccessStatus | null,
  };
  if (!shouldInvokeListSnapshots(token, tokenRef.current, probe)) {
    return {
      ...empty,
      shareStatus: token === tokenRef.current ? probe : null,
    };
  }
  if (!isOk(probe)) return { ...empty, shareStatus: probe };
  if (token !== tokenRef.current) return empty;
  try {
    const rows = await invoke<SnapshotRow[]>("list_snapshots", { repoName });
    if (token !== tokenRef.current) return empty;
    const sorted = sortSnapshotsNewestFirst(rows);
    return {
      snapshots: sorted,
      selected: sorted[0] ?? null,
      snapError: null,
      shareStatus: null,
    };
  } catch (e: unknown) {
    if (token !== tokenRef.current) return empty;
    const msg = String(e);
    if (isShareTimeoutError(msg)) {
      return { ...empty, shareStatus: shareTimeoutStatus() };
    }
    return { ...empty, snapError: msg };
  }
}

export function useSnapshotLoad(selectedRepo: string) {
  const [snapshots, setSnapshots] = useState<SnapshotRow[]>([]);
  const [selectedSnapshot, setSelectedSnapshot] = useState<SnapshotRow | null>(
    null,
  );
  const [snapError, setSnapError] = useState<string | null>(null);
  const [shareStatus, setShareStatus] = useState<ShareAccessStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [reload, setReload] = useState(0);
  const tokenRef = useRef(0);

  const bumpToken = useCallback(() => {
    tokenRef.current += 1;
    return tokenRef.current;
  }, []);

  useEffect(() => {
    if (!selectedRepo) return;
    const token = bumpToken();
    let cancelled = false;
    setSelectedSnapshot(null);
    setSnapshots([]);
    setSnapError(null);
    setShareStatus(null);
    setLoading(true);
    void loadForRepo(selectedRepo, token, tokenRef)
      .then((r) => {
        if (cancelled || token !== tokenRef.current) return;
        setSnapshots(r.snapshots);
        setSelectedSnapshot(r.selected);
        setSnapError(r.snapError);
        setShareStatus(r.shareStatus);
        setLoading(false);
      })
      .catch((e: unknown) => {
        if (cancelled || token !== tokenRef.current) return;
        setSnapError(String(e));
        setShareStatus(null);
        setLoading(false);
      });
    return () => {
      cancelled = true;
      clearListDirCacheForRepo(selectedRepo);
    };
  }, [selectedRepo, reload, bumpToken]);

  const retryList = useCallback(() => {
    bumpToken();
    setReload((n) => n + 1);
  }, [bumpToken]);

  const runSignIn = useCallback(
    async (promptAlternate: boolean) => {
      if (!selectedRepo) return;
      const token = bumpToken();
      setLoading(true);
      setSnapError(null);
      setShareStatus(null);
      try {
        const status = await invoke<ShareAccessStatus>("sign_in_network_share", {
          repoName: selectedRepo,
          promptAlternate,
        });
        if (token !== tokenRef.current) return;
        if (!isOk(status)) {
          setShareStatus(status);
          setLoading(false);
          return;
        }
        if (token !== tokenRef.current) return;
        const r = await loadForRepo(selectedRepo, token, tokenRef);
        if (token !== tokenRef.current) return;
        setSnapshots(r.snapshots);
        setSelectedSnapshot(r.selected);
        setSnapError(r.snapError);
        setShareStatus(r.shareStatus);
      } catch (e: unknown) {
        if (token !== tokenRef.current) return;
        setSnapError(String(e));
      } finally {
        if (token === tokenRef.current) setLoading(false);
      }
    },
    [selectedRepo, bumpToken],
  );

  const selectSnapshotId = useCallback(
    (id: string) => {
      setSelectedSnapshot(snapshots.find((s) => s.id === id) ?? null);
    },
    [snapshots],
  );

  const refreshSelecting = useCallback(
    async (
      preferId: string,
    ): Promise<
      | { status: "selected"; row: SnapshotRow }
      | { status: "missing_id"; expectedId: string }
      | { status: "failed"; message: string }
      | { status: "stale" }
    > => {
      if (!selectedRepo) return { status: "failed", message: "No repository selected." };
      const token = bumpToken();
      setLoading(true);
      setSnapError(null);
      try {
        const r = await loadForRepo(selectedRepo, token, tokenRef);
        if (token !== tokenRef.current) return { status: "stale" };
        setSnapshots(r.snapshots);
        setSnapError(r.snapError);
        setShareStatus(r.shareStatus);
        if (r.snapError) {
          return { status: "failed", message: r.snapError };
        }
        const preferred = r.snapshots.find((s) => s.id === preferId);
        if (!preferred) {
          // Do not fall back to the newest snapshot — tags were saved but the
          // replacement id is missing from the refreshed list.
          setSelectedSnapshot((current) =>
            current && r.snapshots.some((s) => s.id === current.id) ? current : null,
          );
          return { status: "missing_id", expectedId: preferId };
        }
        setSelectedSnapshot(preferred);
        return { status: "selected", row: preferred };
      } catch (e: unknown) {
        if (token !== tokenRef.current) return { status: "stale" };
        const message = String(e);
        setSnapError(message);
        return { status: "failed", message };
      } finally {
        if (token === tokenRef.current) setLoading(false);
      }
    },
    [selectedRepo, bumpToken],
  );

  const prepareRepoSwitch = useCallback(() => {
    bumpToken();
    setSelectedSnapshot(null);
    setSnapshots([]);
    setSnapError(null);
    setShareStatus(null);
    setLoading(true);
  }, [bumpToken]);

  return {
    snapshots,
    selectedSnapshot,
    snapError,
    shareStatus,
    loading,
    retryList,
    selectSnapshotId,
    refreshSelecting,
    prepareRepoSwitch,
    signIn: () => void runSignIn(false),
    tryAnotherAccount: () => void runSignIn(true),
  };
}
