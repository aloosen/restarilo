import { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { beginAction } from "./fileActionInvoke";
import { normalizeTagDraft, tagsEqual } from "./snapshotTags";

export type TagChange = {
  old_snapshot_id: string;
  new_snapshot_id: string;
};

export function useSnapshotTags(opts: {
  repoName: string;
  snapshotId: string | null;
  currentTags: string[];
  actionBusy: boolean;
  onActionId: (id: string | null) => void;
  onBusyLabel: (label: string | null) => void;
  onAfterSave: (change: TagChange) => Promise<void>;
}) {
  const [open, setOpen] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const openEditor = useCallback(() => {
    if (opts.actionBusy || !opts.snapshotId) return;
    setError(null);
    setOpen(true);
  }, [opts.actionBusy, opts.snapshotId]);

  const closeEditor = useCallback(() => {
    if (saving) return;
    setOpen(false);
    setError(null);
  }, [saving]);

  const save = useCallback(
    async (draft: string[]) => {
      if (saving || !opts.snapshotId) return;
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
      try {
        const actionId = await beginAction("tags", {
          onActionId: opts.onActionId,
          onBusyLabel: opts.onBusyLabel,
          onSuccess: () => {},
          onError: () => {},
        });
        const change = await invoke<TagChange>("set_snapshot_tags", {
          repoName: opts.repoName,
          snapshotId: opts.snapshotId,
          tags: n.tags,
          actionId,
        });
        await opts.onAfterSave(change);
        setOpen(false);
        setError(null);
      } catch (e: unknown) {
        setError(String(e));
      } finally {
        opts.onActionId(null);
        opts.onBusyLabel(null);
        setSaving(false);
      }
    },
    [saving, opts],
  );

  return { open, saving, error, openEditor, closeEditor, save };
}
