import {
  joinTarget,
  successMessage,
  targetFromExistsError,
  type ActionKind,
} from "./actionFeedback";
import {
  beginAction,
  invokeAction,
  pickFolder,
  selectionIdentityMatches,
  type FileSelection,
} from "./fileActionInvoke";
import { FileActionButtons } from "./FileActionButtons";
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
  };

  function end() {
    p.onActionId?.(null);
    p.onBusyLabel?.(null);
  }

  function matches(frozen: {
    repoName: string;
    snapshotId: string;
    selection: FileSelection;
  }) {
    return selectionIdentityMatches(
      { repoName: p.repoName, snapshotId: p.snapshotId, selection: p.selection },
      frozen,
    );
  }

  async function runRestoreOrDump(cmd: "restore_path" | "dump_path") {
    if (!p.selection) return;
    const frozen = {
      repoName: p.repoName,
      snapshotId: p.snapshotId,
      snapshotLabel: p.snapshotLabel,
      selection: { ...p.selection },
    };
    const targetDir = await pickFolder();
    if (!targetDir) return;
    if (!matches(frozen)) return;

    const kind: ActionKind = cmd === "restore_path" ? "restore" : "export";
    const expected = joinTarget(
      targetDir,
      frozen.selection.name,
      cmd === "dump_path" && frozen.selection.is_dir,
    );
    const base = {
      repoName: frozen.repoName,
      snapshotId: frozen.snapshotId,
      path: frozen.selection.path,
      name: frozen.selection.name,
      targetDir,
    };

    async function attempt(overwrite: boolean) {
      if (!matches(frozen)) return;
      let id: string;
      try {
        id = await beginAction(kind, hooks);
      } catch (e: unknown) {
        p.onError?.(String(e));
        return;
      }
      try {
        const payload =
          cmd === "dump_path"
            ? { ...base, isDir: frozen.selection.is_dir, overwrite }
            : { ...base, overwrite };
        const dest = await invokeAction(cmd, payload, id, () => {
          p.onSuccess?.(null);
          p.onError?.(null);
        });
        p.onSuccess?.(successMessage(kind, dest || expected));
      } catch (e: unknown) {
        const msg = String(e);
        end();
        if (msg.includes("CANCELLED:")) return;
        if (msg.includes("TARGET_EXISTS:") && !overwrite) {
          p.onOverwriteOpenChange?.(true);
          let ok = false;
          try {
            ok = await askOverwrite({
              sourcePath: frozen.selection.path,
              snapshotLabel: frozen.snapshotLabel,
              targetPath: targetFromExistsError(msg) ?? expected,
            });
          } finally {
            p.onOverwriteOpenChange?.(false);
          }
          if (!ok || !matches(frozen)) return;
          await attempt(true);
          return;
        }
        p.onError?.(msg);
        return;
      }
      end();
    }

    await attempt(false);
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
      if (!matches(frozen)) return;
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
        onRestore={() => void runRestoreOrDump("restore_path")}
        onExport={() => void runRestoreOrDump("dump_path")}
        onOpen={() => void onOpen()}
      />
      {modal}
    </>
  );
}
