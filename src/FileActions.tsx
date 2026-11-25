import { useState } from "react";
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
  onError?: (message: string | null) => void;
};

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
  onError,
}: Props) {
  const [busy, setBusy] = useState(false);
  const noSel = !selection;
  const block = disabled || busy;

  async function invokeWithOverwrite(
    kind: "restore_path" | "dump_path",
    targetDir: string,
  ) {
    if (!selection) return;
    const base = {
      repoName,
      snapshotId,
      path: selection.path,
      name: selection.name,
      targetDir,
    };
    const payload =
      kind === "dump_path"
        ? { ...base, isDir: selection.is_dir, overwrite: false }
        : { ...base, overwrite: false };

    const tryOnce = (overwrite: boolean) =>
      invoke(
        kind,
        kind === "dump_path"
          ? { ...payload, overwrite }
          : { ...base, overwrite },
      );

    try {
      await tryOnce(false);
    } catch (e: unknown) {
      const msg = String(e);
      if (!msg.includes("TARGET_EXISTS:")) throw e;
      const ok = await ask("Target already exists. Overwrite?", {
        title: "Overwrite?",
        kind: "warning",
      });
      if (!ok) return;
      await tryOnce(true);
    }
  }

  async function onRestoreOrDump(kind: "restore_path" | "dump_path") {
    if (!selection) return;
    const targetDir = await pickFolder();
    if (!targetDir) return;
    onError?.(null);
    setBusy(true);
    try {
      await invokeWithOverwrite(kind, targetDir);
    } catch (e: unknown) {
      onError?.(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function onOpen() {
    if (!selection || selection.is_dir) return;
    onError?.(null);
    setBusy(true);
    try {
      await invoke("open_path", {
        repoName,
        snapshotId,
        path: selection.path,
        name: selection.name,
        isDir: false,
      });
    } catch (e: unknown) {
      onError?.(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="file-actions">
      <button
        type="button"
        disabled={block || noSel}
        onClick={() => void onRestoreOrDump("restore_path")}
      >
        Restore
      </button>
      <button
        type="button"
        disabled={block || noSel}
        onClick={() => void onRestoreOrDump("dump_path")}
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
      {busy && <span className="file-actions-busy">Working…</span>}
    </div>
  );
}
