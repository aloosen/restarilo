import { useCallback, useState } from "react";
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

export function useSearchAppWire(opts: {
  enabled: boolean;
  repoName: string;
  snapshotId: string | null;
  snapshots: SnapshotRow[];
  actionBusy: boolean;
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
  const [matches, setMatches] = useState<SearchHit[] | null>(null);
  const [truncated, setTruncated] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [searching, setSearching] = useState(false);

  const onOpen = useCallback(() => setOpen(true), []);
  const onCloseIdle = useCallback(() => setOpen(false), []);

  useSearchHotkey({
    enabled: opts.enabled,
    actionBusy: opts.actionBusy,
    open,
    onOpen,
    onCloseIdle,
  });

  async function onSearch() {
    if (!opts.repoName || opts.actionBusy || searching) return;
    const q = query.trim();
    if (!q) return;
    setSearching(true);
    setError(null);
    setMatches(null);
    setTruncated(false);
    try {
      const snap =
        mode === "current" ? opts.snapshotId : null;
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
          onSuccess: () => {},
          onError: () => {},
        },
      });
      const capped = capMatches(all);
      setMatches(capped.shown);
      setTruncated(capped.truncated);
    } catch (e: unknown) {
      const msg = String(e);
      if (msg.includes("CANCELLED:")) {
        setError(null);
        return;
      }
      setError(msg);
    } finally {
      setSearching(false);
    }
  }

  function onExportHit(hit: SearchHit) {
    if (opts.actionBusy || !opts.repoName) return;
    void exportSearchHit({
      repoName: opts.repoName,
      hit,
      snapshotLabel: labelForSnapshot(hit.snapshot_id, opts.snapshots),
      hooks: {
        onActionId: opts.onActionId,
        onBusyLabel: opts.onBusyLabel,
        onOverwriteOpenChange: opts.onOverwriteOpenChange,
        onSuccess: opts.onSuccess,
        onError: opts.onError,
      },
      askOverwrite,
    });
  }

  const overlay = (
    <>
      <SearchOverlay
        open={open}
        mode={mode}
        query={query}
        actionBusy={opts.actionBusy}
        matches={matches}
        truncated={truncated}
        error={error}
        searching={searching}
        labelForSnapshot={(id) => labelForSnapshot(id, opts.snapshots)}
        onClose={onCloseIdle}
        onModeChange={setMode}
        onQueryChange={setQuery}
        onSearch={() => void onSearch()}
        onShowHit={opts.onShowHit}
        onExportHit={onExportHit}
      />
      {overwriteModal}
    </>
  );

  return { overlay, open, setOpen };
}
