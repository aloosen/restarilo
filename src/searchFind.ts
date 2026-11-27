import { invoke } from "@tauri-apps/api/core";
import { beginAction } from "./fileActionInvoke";
import { formatSnapshotOption } from "./formatSize";
import type { SnapshotRow } from "./SnapshotList";
import type { SearchHit } from "./SearchOverlay";

export const SEARCH_DISPLAY_CAP = 200;

type FindPathsResult = { matches: SearchHit[] };

export function labelForSnapshot(
  id: string,
  snapshots: SnapshotRow[],
): string {
  const row = snapshots.find((s) => s.id === id);
  if (row) return formatSnapshotOption(row).text;
  return id.slice(0, 8);
}

export async function runFindSearch(opts: {
  repoName: string;
  query: string;
  snapshotId: string | null;
  hooks: {
    onSuccess?: (message: string | null) => void;
    onError?: (message: string | null) => void;
    onActionId?: (id: string | null) => void;
    onBusyLabel?: (label: string | null) => void;
  };
}): Promise<SearchHit[]> {
  const id = await beginAction("search", opts.hooks);
  try {
    const result = await invoke<FindPathsResult>("find_paths", {
      repoName: opts.repoName,
      query: opts.query,
      snapshotId: opts.snapshotId,
      actionId: id,
    });
    opts.hooks.onActionId?.(null);
    opts.hooks.onBusyLabel?.(null);
    return result.matches;
  } catch (e: unknown) {
    const msg = String(e);
    opts.hooks.onActionId?.(null);
    opts.hooks.onBusyLabel?.(null);
    if (msg.includes("CANCELLED:")) {
      opts.hooks.onSuccess?.(null);
      opts.hooks.onError?.(null);
      throw e;
    }
    throw e;
  }
}

export function capMatches(matches: SearchHit[]): {
  shown: SearchHit[];
  truncated: boolean;
} {
  if (matches.length > SEARCH_DISPLAY_CAP) {
    return {
      shown: matches.slice(0, SEARCH_DISPLAY_CAP),
      truncated: true,
    };
  }
  return { shown: matches, truncated: false };
}
