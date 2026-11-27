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

export type PathActionHooks = {
  onSuccess?: (message: string | null) => void;
  onError?: (message: string | null) => void;
  onActionId?: (id: string | null) => void;
  onBusyLabel?: (label: string | null) => void;
  onOverwriteOpenChange?: (open: boolean) => void;
};

export type PathActionTarget = {
  repoName: string;
  snapshotId: string;
  snapshotLabel: string;
  selection: FileSelection;
};

export async function runRestoreOrDump(opts: {
  cmd: "restore_path" | "dump_path";
  frozen: PathActionTarget;
  live: {
    repoName: string;
    snapshotId: string;
    selection: FileSelection | null;
  };
  hooks: PathActionHooks;
  askOverwrite: (args: {
    sourcePath: string;
    snapshotLabel: string;
    targetPath: string;
  }) => Promise<boolean>;
}): Promise<void> {
  const { cmd, frozen, live, hooks, askOverwrite } = opts;
  const targetDir = await pickFolder();
  if (!targetDir) return;
  if (!selectionIdentityMatches(live, frozen)) return;

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

  function end() {
    hooks.onActionId?.(null);
    hooks.onBusyLabel?.(null);
  }

  async function attempt(overwrite: boolean) {
    if (!selectionIdentityMatches(live, frozen)) return;
    let id: string;
    try {
      id = await beginAction(kind, hooks);
    } catch (e: unknown) {
      hooks.onError?.(String(e));
      return;
    }
    try {
      const payload =
        cmd === "dump_path"
          ? { ...base, isDir: frozen.selection.is_dir, overwrite }
          : { ...base, overwrite };
      const dest = await invokeAction(cmd, payload, id, () => {
        hooks.onSuccess?.(null);
        hooks.onError?.(null);
      });
      hooks.onSuccess?.(successMessage(kind, dest || expected));
    } catch (e: unknown) {
      const msg = String(e);
      end();
      if (msg.includes("CANCELLED:")) return;
      if (msg.includes("TARGET_EXISTS:") && !overwrite) {
        hooks.onOverwriteOpenChange?.(true);
        let ok = false;
        try {
          ok = await askOverwrite({
            sourcePath: frozen.selection.path,
            snapshotLabel: frozen.snapshotLabel,
            targetPath: targetFromExistsError(msg) ?? expected,
          });
        } finally {
          hooks.onOverwriteOpenChange?.(false);
        }
        if (!ok || !selectionIdentityMatches(live, frozen)) return;
        await attempt(true);
        return;
      }
      hooks.onError?.(msg);
      return;
    }
    end();
  }

  await attempt(false);
}
