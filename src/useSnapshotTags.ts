import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { beginAction } from "./fileActionInvoke";
import { normalizeTagDraft, tagsEqual } from "./snapshotTags";

export type TagChange = {
  old_snapshot_id: string;
  new_snapshot_id: string;
};

export type TagRefreshResult =
  | { status: "ok" }
  | { status: "needs_reload"; message: string };

const DEFAULT_RELOAD_MSG =
  "Tags were saved, but the updated snapshot could not be loaded yet. Reload the snapshot list to continue.";

export function useSnapshotTags(opts: {
  repoName: string;
  snapshotId: string | null;
  currentTags: string[];
  actionBusy: boolean;
  onActionId: (id: string | null) => void;
  onBusyLabel: (label: string | null) => void;
  onAfterSave: (change: TagChange) => Promise<TagRefreshResult>;
  onReloadSnapshot: (snapshotId: string) => Promise<TagRefreshResult>;
}) {
  const [open, setOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [reloading, setReloading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [pendingNewId, setPendingNewId] = useState<string | null>(null);

  const openEditor = useCallback(() => {
    if (opts.actionBusy || !opts.snapshotId) return;
    setError(null);
    setPendingNewId(null);
    setOpen(true);
  }, [opts.actionBusy, opts.snapshotId]);

  const closeEditor = useCallback(() => {
    if (saving || reloading) return;
    setOpen(false);
    setError(null);
    setPendingNewId(null);
  }, [saving, reloading]);

  const save = useCallback(
    async (draft: string[]) => {
      if (saving || reloading || pendingNewId || !opts.snapshotId) return;
      const n = normalizeTagDraft(draft);
      if (n.error) {
        setError(n.error);
        return;
      }
      if (tagsEqual(n.tags, opts.currentTags)) {
        setOpen(false);
        setError(null);
        return;
      }
      setSaving(true);
      setError(null);
      let actionId: string | null = null;
      try {
        actionId = await beginAction("tags", {
          onActionId: opts.onActionId,
          onBusyLabel: opts.onBusyLabel,
          onSuccess: () => {},
          onError: () => {},
        });
        const change = await invoke<TagChange>("set_snapshot_tags", {
          repoName: opts.repoName,
          snapshotId: opts.snapshotId,
          tags: n.tags,
          previousTags: opts.currentTags,
          actionId,
        });
        // Hold the new id before refresh so the explorer is not unmounted when
        // the old snapshot disappears from the list.
        setPendingNewId(change.new_snapshot_id);
        const refreshed = await opts.onAfterSave(change);
        if (refreshed.status === "needs_reload") {
          setError(refreshed.message || DEFAULT_RELOAD_MSG);
          return;
        }
        setPendingNewId(null);
        setOpen(false);
        setError(null);
      } catch (e: unknown) {
        setError(String(e).replace(/^Error:\s*/i, ""));
      } finally {
        if (actionId) {
          void invoke("cancel_action", { actionId }).catch(() => {});
        }
        opts.onActionId(null);
        opts.onBusyLabel(null);
        setSaving(false);
      }
    },
    [saving, reloading, pendingNewId, opts],
  );

  const reloadPending = useCallback(async () => {
    if (!pendingNewId || saving || reloading) return;
    setReloading(true);
    setError(null);
    try {
      const refreshed = await opts.onReloadSnapshot(pendingNewId);
      if (refreshed.status === "needs_reload") {
        setError(refreshed.message || DEFAULT_RELOAD_MSG);
        return;
      }
      setPendingNewId(null);
      setOpen(false);
      setError(null);
    } catch (e: unknown) {
      setError(String(e).replace(/^Error:\s*/i, ""));
    } finally {
      setReloading(false);
    }
  }, [pendingNewId, saving, reloading, opts]);

  return {
    open,
    saving,
    reloading,
    error,
    pendingNewId,
    openEditor,
    closeEditor,
    save,
    reloadPending,
  };
}
