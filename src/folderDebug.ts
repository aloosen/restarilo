import { invoke } from "@tauri-apps/api/core";

/** Append a line to the opt-in Rust debug log (`folder_debug_fe`). No-op on failure. */
export function folderDebug(message: string): void {
  void invoke("folder_debug_fe", { message }).catch(() => {});
}

/** Resolve the debug log file path (creates/truncates a session on first use). */
export function folderDebugPath(): Promise<string> {
  return invoke<string>("folder_debug_path");
}
