import { successMessage } from "./actionFeedback";
import {
  beginAction,
  invokeAction,
  selectionIdentityMatches,
  type FileSelection,
} from "./fileActionInvoke";
import { FileActionButtons } from "./FileActionButtons";
import { runRestoreOrDump } from "./pathActionRun";
import { useOverwriteAsk } from "./useOverwriteAsk";

export type { FileSelection };

type Props = {
  repoName: string;
  snapshotId: string;
  snapshotLabel: string;
  selection: FileSelection | null;
  disabled?: boolean;
  actionBusy?: boolean;
  onActionId?: (id: string | null) => void;
  onBusyLabel?: (label: string | null) => void;
  onOverwriteOpenChange?: (open: boolean) => void;
  onSuccess?: (message: string | null) => void;
  onError?: (message: string | null) => void;
};

export function FileActions(p: Props) {
  const block = p.disabled || !!p.actionBusy;
  const { askOverwrite, modal } = useOverwriteAsk();
  const hooks = {
    onSuccess: p.onSuccess,
    onError: p.onError,
    onActionId: p.onActionId,
    onBusyLabel: p.onBusyLabel,
    onOverwriteOpenChange: p.onOverwriteOpenChange,
  };

  function end() {
    p.onActionId?.(null);
    p.onBusyLabel?.(null);
  }

  async function onRestoreOrDump(cmd: "restore_path" | "dump_path") {
    if (!p.selection) return;
    await runRestoreOrDump({
      cmd,
      frozen: {
        repoName: p.repoName,
        snapshotId: p.snapshotId,
        snapshotLabel: p.snapshotLabel,
        selection: { ...p.selection },
      },
      live: {
        repoName: p.repoName,
        snapshotId: p.snapshotId,
        selection: p.selection,
      },
      hooks,
      askOverwrite,
    });
  }

  async function onOpen() {
    if (!p.selection || p.selection.is_dir) return;
    const frozen = {
      repoName: p.repoName,
      snapshotId: p.snapshotId,
      selection: { ...p.selection },
    };
    let id: string;
    try {
      id = await beginAction("open", hooks);
    } catch (e: unknown) {
      p.onError?.(String(e));
      return;
    }
    try {
      if (
        !selectionIdentityMatches(
          { repoName: p.repoName, snapshotId: p.snapshotId, selection: p.selection },
          frozen,
        )
      ) {
        return;
      }
      const dest = await invokeAction(
        "open_path",
        {
          repoName: frozen.repoName,
          snapshotId: frozen.snapshotId,
          path: frozen.selection.path,
          name: frozen.selection.name,
          isDir: false,
        },
        id,
        () => {
          p.onSuccess?.(null);
          p.onError?.(null);
        },
      );
      p.onSuccess?.(successMessage("open", dest || frozen.selection.name));
    } catch (e: unknown) {
      const msg = String(e);
      if (!msg.includes("CANCELLED:")) p.onError?.(msg);
    } finally {
      end();
    }
  }

  const noSel = block || !p.selection;
  return (
    <>
      <FileActionButtons
        disabled={noSel}
        openDisabled={noSel || !!p.selection?.is_dir}
        onRestore={() => void onRestoreOrDump("restore_path")}
        onExport={() => void onRestoreOrDump("dump_path")}
        onOpen={() => void onOpen()}
      />
      {modal}
    </>
  );
}
