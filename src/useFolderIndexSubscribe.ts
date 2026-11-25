import { useEffect, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import {
  mergeFolderDirs,
  subscribeFolderIndex,
  type FolderIndexUpdated,
} from "./folderIndex";
import { mergeBranchFromEvent, type BranchState } from "./folderTreeNav";

/** Subscribe to folder-index events with alive-safe unlisten. */
export function useFolderIndexSubscribe(
  repoName: string,
  snapshotId: string,
  activeGeneration: MutableRefObject<number | null>,
  identityRef: MutableRefObject<string>,
  setBranches: Dispatch<SetStateAction<Record<string, BranchState>>>,
): void {
  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    void subscribeFolderIndex(
      repoName,
      snapshotId,
      () => activeGeneration.current,
      (ev: FolderIndexUpdated) => {
        if (!alive || identityRef.current !== `${repoName}\0${snapshotId}`) return;
        setBranches((prev) => {
          const cur = prev[ev.parentPath];
          const dirs = mergeFolderDirs(cur?.dirs ?? [], ev.dirs);
          return {
            ...prev,
            [ev.parentPath]: mergeBranchFromEvent(cur, dirs, ev.childrenComplete),
          };
        });
      },
    ).then((u) => {
      if (!alive) {
        u();
        return;
      }
      unlisten = u;
    });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, [repoName, snapshotId, activeGeneration, identityRef, setBranches]);
}
