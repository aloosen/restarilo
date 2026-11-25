import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { busyLabelFor, type ActionKind } from "./actionFeedback";

export type FileSelection = {
  path: string;
  name: string;
  is_dir: boolean;
};

export async function pickFolder(): Promise<string | null> {
  const dir = await open({ directory: true, multiple: false });
  return dir === null ? null : typeof dir === "string" ? dir : null;
}

export function selectionIdentityMatches(
  live: {
    repoName: string;
    snapshotId: string;
    selection: FileSelection | null;
  },
  frozen: { repoName: string; snapshotId: string; selection: FileSelection },
): boolean {
  return (
    live.repoName === frozen.repoName &&
    live.snapshotId === frozen.snapshotId &&
    live.selection?.path === frozen.selection.path &&
    live.selection?.name === frozen.selection.name &&
    live.selection?.is_dir === frozen.selection.is_dir
  );
}

export async function beginAction(
  kind: ActionKind,
  hooks: {
    onSuccess?: (message: string | null) => void;
    onError?: (message: string | null) => void;
    onActionId?: (id: string | null) => void;
    onBusyLabel?: (label: string | null) => void;
  },
): Promise<string> {
  hooks.onSuccess?.(null);
  hooks.onError?.(null);
  const id = crypto.randomUUID();
  await invoke("begin_action", { actionId: id });
  hooks.onActionId?.(id);
  hooks.onBusyLabel?.(busyLabelFor(kind));
  return id;
}

export async function invokeAction(
  cmd: "restore_path" | "dump_path" | "open_path",
  payload: Record<string, unknown>,
  actionId: string,
  onClearFeedback?: () => void,
): Promise<string> {
  try {
    return await invoke<string>(cmd, { ...payload, actionId });
  } catch (e: unknown) {
    if (String(e).includes("CANCELLED:")) onClearFeedback?.();
    throw e;
  }
}
