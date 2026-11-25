import { invoke } from "@tauri-apps/api/core";
import { ask, open } from "@tauri-apps/plugin-dialog";

export type FileSelection = {
  path: string;
  name: string;
  is_dir: boolean;
};

type Props = {
  repoName: string;
  snapshotId: string;
  selection: FileSelection | null;
  disabled?: boolean;
  actionBusy?: boolean;
  onActionId?: (id: string | null) => void;
  onError?: (message: string | null) => void;
};

function newActionId(): string {
  return crypto.randomUUID();
}

async function pickFolder(): Promise<string | null> {
  const dir = await open({ directory: true, multiple: false });
  if (dir === null) return null;
  return typeof dir === "string" ? dir : null;
}

export function FileActions({
  repoName,
  snapshotId,
  selection,
  disabled,
  actionBusy,
  onActionId,
  onError,
}: Props) {
  const noSel = !selection;
  const block = disabled || !!actionBusy;

  async function beginSession(): Promise<string> {
    const id = newActionId();
    await invoke("begin_action", { actionId: id });
    onActionId?.(id);
    return id;
  }

  function endSession() {
    onActionId?.(null);
  }

  async function invokeAction(
    kind: "restore_path" | "dump_path" | "open_path",
    payload: Record<string, unknown>,
    actionId: string,
  ) {
    try {
      await invoke(kind, { ...payload, actionId });
    } catch (e: unknown) {
      const msg = String(e);
      if (msg.includes("CANCELLED:")) return;
      throw e;
    }
  }

  async function runRestoreOrDump(kind: "restore_path" | "dump_path") {
    if (!selection) return;
    const targetDir = await pickFolder();
    if (!targetDir) return;
    onError?.(null);

    const base = {
      repoName,
      snapshotId,
      path: selection.path,
      name: selection.name,
      targetDir,
    };

    async function attempt(overwrite: boolean) {
      let id: string;
      try {
        id = await beginSession();
      } catch (e: unknown) {
        onError?.(String(e));
        return;
      }
      try {
        const payload =
          kind === "dump_path"
            ? { ...base, isDir: selection!.is_dir, overwrite }
            : { ...base, overwrite };
        await invokeAction(kind, payload, id);
      } catch (e: unknown) {
        const msg = String(e);
        endSession();
        if (msg.includes("TARGET_EXISTS:") && !overwrite) {
          const ok = await ask("Target already exists. Overwrite?", {
            title: "Overwrite?",
            kind: "warning",
          });
          if (!ok) return;
          await attempt(true);
          return;
        }
        onError?.(msg);
        return;
      }
      endSession();
    }

    await attempt(false);
  }

  async function onOpen() {
    if (!selection || selection.is_dir) return;
    onError?.(null);
    let id: string;
    try {
      id = await beginSession();
    } catch (e: unknown) {
      onError?.(String(e));
      return;
    }
    try {
      await invokeAction(
        "open_path",
        {
          repoName,
          snapshotId,
          path: selection.path,
          name: selection.name,
          isDir: false,
        },
        id,
      );
    } catch (e: unknown) {
      onError?.(String(e));
    } finally {
      endSession();
    }
  }

  return (
    <div className="file-actions">
      <button
        type="button"
        disabled={block || noSel}
        onClick={() => void runRestoreOrDump("restore_path")}
      >
        Restore
      </button>
      <button
        type="button"
        disabled={block || noSel}
        onClick={() => void runRestoreOrDump("dump_path")}
      >
        Dump
      </button>
      <button
        type="button"
        disabled={block || noSel || !!selection?.is_dir}
        onClick={() => void onOpen()}
      >
        Open
      </button>
    </div>
  );
}
