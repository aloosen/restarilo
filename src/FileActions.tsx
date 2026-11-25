import { invoke } from "@tauri-apps/api/core";
import { ask, open } from "@tauri-apps/plugin-dialog";
import {
  busyLabelFor,
  joinTarget,
  overwriteAskBody,
  successMessage,
  targetFromExistsError,
  type ActionKind,
} from "./actionFeedback";

export type FileSelection = {
  path: string;
  name: string;
  is_dir: boolean;
};

type Props = {
  repoName: string;
  snapshotId: string;
  snapshotLabel: string;
  selection: FileSelection | null;
  disabled?: boolean;
  actionBusy?: boolean;
  onActionId?: (id: string | null) => void;
  onBusyLabel?: (label: string | null) => void;
  onSuccess?: (message: string | null) => void;
  onError?: (message: string | null) => void;
};

async function pickFolder(): Promise<string | null> {
  const dir = await open({ directory: true, multiple: false });
  return dir === null ? null : typeof dir === "string" ? dir : null;
}

export function FileActions(p: Props) {
  const { selection } = p;
  const block = p.disabled || !!p.actionBusy;

  async function begin(kind: ActionKind): Promise<string> {
    p.onSuccess?.(null);
    p.onError?.(null);
    const id = crypto.randomUUID();
    await invoke("begin_action", { actionId: id });
    p.onActionId?.(id);
    p.onBusyLabel?.(busyLabelFor(kind));
    return id;
  }

  function end() {
    p.onActionId?.(null);
    p.onBusyLabel?.(null);
  }

  async function call(
    cmd: "restore_path" | "dump_path" | "open_path",
    payload: Record<string, unknown>,
    actionId: string,
  ): Promise<string> {
    try {
      return await invoke<string>(cmd, { ...payload, actionId });
    } catch (e: unknown) {
      if (String(e).includes("CANCELLED:")) {
        p.onSuccess?.(null);
        p.onError?.(null);
      }
      throw e;
    }
  }

  async function runRestoreOrDump(cmd: "restore_path" | "dump_path") {
    if (!selection) return;
    const targetDir = await pickFolder();
    if (!targetDir) return;
    const kind: ActionKind = cmd === "restore_path" ? "restore" : "export";
    const expected = joinTarget(
      targetDir,
      selection.name,
      cmd === "dump_path" && selection.is_dir,
    );
    const base = {
      repoName: p.repoName,
      snapshotId: p.snapshotId,
      path: selection.path,
      name: selection.name,
      targetDir,
    };

    async function attempt(overwrite: boolean) {
      let id: string;
      try {
        id = await begin(kind);
      } catch (e: unknown) {
        p.onError?.(String(e));
        return;
      }
      try {
        const payload =
          cmd === "dump_path"
            ? { ...base, isDir: selection!.is_dir, overwrite }
            : { ...base, overwrite };
        const dest = await call(cmd, payload, id);
        p.onSuccess?.(successMessage(kind, dest || expected));
      } catch (e: unknown) {
        const msg = String(e);
        end();
        if (msg.includes("CANCELLED:")) return;
        if (msg.includes("TARGET_EXISTS:") && !overwrite) {
          const target = targetFromExistsError(msg) ?? expected;
          const ok = await ask(
            overwriteAskBody({
              sourcePath: selection!.path,
              snapshotLabel: p.snapshotLabel,
              targetPath: target,
            }),
            { title: "Overwrite?", kind: "warning" },
          );
          if (!ok) return;
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
    if (!selection || selection.is_dir) return;
    let id: string;
    try {
      id = await begin("open");
    } catch (e: unknown) {
      p.onError?.(String(e));
      return;
    }
    try {
      const dest = await call(
        "open_path",
        {
          repoName: p.repoName,
          snapshotId: p.snapshotId,
          path: selection.path,
          name: selection.name,
          isDir: false,
        },
        id,
      );
      p.onSuccess?.(successMessage("open", dest || selection.name));
    } catch (e: unknown) {
      const msg = String(e);
      if (!msg.includes("CANCELLED:")) p.onError?.(msg);
    } finally {
      end();
    }
  }

  return (
    <div className="file-actions">
      <button
        type="button"
        className="file-actions-primary"
        disabled={block || !selection}
        onClick={() => void runRestoreOrDump("restore_path")}
      >
        Restore
      </button>
      <button
        type="button"
        disabled={block || !selection}
        onClick={() => void runRestoreOrDump("dump_path")}
      >
        Export
      </button>
      <button
        type="button"
        disabled={block || !selection || !!selection?.is_dir}
        onClick={() => void onOpen()}
      >
        Open
      </button>
    </div>
  );
}
