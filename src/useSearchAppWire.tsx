import { useCallback, useEffect, useRef, useState } from "react";
import {
  SearchOverlay,
  type SearchHit,
  type SearchMode,
} from "./SearchOverlay";
import { exportSearchHit } from "./searchExport";
import {
  capMatches,
  labelForSnapshot,
  runFindSearch,
} from "./searchFind";
import type { SnapshotRow } from "./SnapshotList";
import { useOverwriteAsk } from "./useOverwriteAsk";
import { useSearchHotkey } from "./useSearchHotkey";
import type { useActionCancel } from "./useActionCancel";

export function useSearchAppWire(opts: {
  enabled: boolean;
  repoName: string;
  snapshotId: string | null;
  snapshots: SnapshotRow[];
  actionBusy: boolean;
  overwriteOpen: boolean;
  /** Extra UI lock (e.g. tag editor) — blocks search hotkey and actions. */
  uiBlocked?: boolean;
  cancellation: ReturnType<typeof useActionCancel>;
  onActionId: (id: string | null) => void;
  onBusyLabel: (label: string | null) => void;
  onOverwriteOpenChange?: (open: boolean) => void;
  onSuccess?: (message: string | null) => void;
  onError?: (message: string | null) => void;
  onShowHit?: (hit: SearchHit) => void;
}) {
  const { askOverwrite, modal: overwriteModal } = useOverwriteAsk();
  const [open, setOpen] = useState(false);
  const [mode, setMode] = useState<SearchMode>("current");
  const [query, setQuery] = useState("");
  const [result, setResult] = useState<ReturnType<typeof capMatches> & {
    repoName: string;
    mode: SearchMode;
  }>();
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [searching, setSearching] = useState(false);
  const [cancelled, setCancelled] = useState(false);
  const input = useRef<HTMLInputElement>(null);

  const onOpen = useCallback(() => {
    setOpen(true);
    input.current?.focus();
  }, []);
  const onCloseIdle = useCallback(() => setOpen(false), []);

  const uiBlocked = !!opts.uiBlocked;
  useSearchHotkey({
    enabled: opts.enabled,
    blocked: opts.actionBusy || opts.overwriteOpen || uiBlocked || searching,
    open,
    onOpen,
    onCloseIdle,
  });

  useEffect(() => {
    setResult(undefined);
    setError(null);
    setNotice(null);
    setCancelled(false);
  }, [opts.repoName]);

  const q = query.trim();
  const covered =
    opts.overwriteOpen || uiBlocked || (opts.actionBusy && !searching);
  const canSearch = opts.enabled && !covered && !searching && /[^/]/.test(q);

  async function onSearch() {
    if (!canSearch) return;
    setSearching(true);
    setError(null);
    setNotice(null);
    setResult(undefined);
    setCancelled(false);
    try {
      const snap = mode === "current" ? opts.snapshotId : null;
      if (mode === "current" && !snap) {
        setError("No snapshot selected");
        return;
      }
      const all = await runFindSearch({
        repoName: opts.repoName,
        query: q,
        snapshotId: snap,
        hooks: {
          onActionId: opts.onActionId,
          onBusyLabel: opts.onBusyLabel,
        },
      });
      setResult({ ...capMatches(all), repoName: opts.repoName, mode });
    } catch (e: unknown) {
      const msg = String(e);
      if (msg.includes("CANCELLED:")) {
        setCancelled(true);
        return;
      }
      setError(msg);
    } finally {
      setSearching(false);
    }
  }

  function onExportHit(hit: SearchHit) {
    if (covered || searching || result?.repoName !== opts.repoName) return;
    void exportSearchHit({
      repoName: opts.repoName,
      hit,
      snapshotLabel: labelForSnapshot(hit.snapshot_id, opts.snapshots),
      hooks: {
        onActionId: opts.onActionId,
        onBusyLabel: opts.onBusyLabel,
        onOverwriteOpenChange: opts.onOverwriteOpenChange,
        onSuccess: (message) => {
          setNotice(message);
          setError(null);
          opts.onSuccess?.(message);
        },
        onError: (message) => {
          // Keep matches visible so Show/Export stay available after a failed export.
          setNotice(null);
          setError(message);
          opts.onError?.(message);
        },
      },
      askOverwrite,
    });
  }

  function onShowHit(hit: SearchHit) {
    setOpen(false);
    opts.onShowHit?.(hit);
  }

  const remapSnapshotId = useCallback((oldId: string, newId: string) => {
    if (oldId === newId) return;
    setResult((prev) => {
      if (!prev) return prev;
      return {
        ...prev,
        shown: prev.shown.map((h) =>
          h.snapshot_id === oldId ? { ...h, snapshot_id: newId } : h,
        ),
      };
    });
  }, []);

  const visibleResult = result?.repoName === opts.repoName ? result : undefined;

  const overlay = (
    <>
      <SearchOverlay
        open={open}
        mode={mode}
        query={query}
        covered={covered}
        canSearch={canSearch}
        input={input}
        matches={visibleResult?.shown ?? null}
        truncated={visibleResult?.truncated ?? false}
        showSnapshotLabels={visibleResult?.mode === "all"}
        error={error}
        notice={notice}
        searching={searching}
        cancelled={cancelled}
        canCancel={searching && opts.actionBusy}
        {...opts.cancellation}
        labelForSnapshot={(id) => labelForSnapshot(id, opts.snapshots)}
        onClose={onCloseIdle}
        onModeChange={setMode}
        onQueryChange={setQuery}
        onSearch={() => void onSearch()}
        onShowHit={onShowHit}
        onExportHit={onExportHit}
      />
      {overwriteModal}
    </>
  );

  return { overlay, open, searching, setOpen, remapSnapshotId };
}
