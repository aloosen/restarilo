import { useCallback, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { ExplorerListing } from "./ExplorerListing";
import { FolderTree } from "./FolderTree";
import type { DirEntry } from "./listDirCache";
import { useExplorerBrowse } from "./useExplorerBrowse";

export type { DirEntry };

const TREE_MIN = 128;
const TREE_DEFAULT = 224;

type Props = {
  repoName: string;
  snapshotId: string;
  snapshotLabel: string;
  actionBusy?: boolean;
  onActionId?: (id: string | null) => void;
  onBusyLabel?: (label: string | null) => void;
};

export function ExplorerView({
  repoName,
  snapshotId,
  snapshotLabel,
  actionBusy = false,
  onActionId,
  onBusyLabel,
}: Props) {
  const b = useExplorerBrowse(repoName, snapshotId);
  const [treeWidth, setTreeWidth] = useState(TREE_DEFAULT);
  const drag = useRef<{ startX: number; startW: number } | null>(null);

  const onSplitPointerDown = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    drag.current = { startX: e.clientX, startW: treeWidth };
    e.currentTarget.setPointerCapture(e.pointerId);
  }, [treeWidth]);

  const onSplitPointerMove = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    const root = e.currentTarget.parentElement;
    const max = root ? Math.max(TREE_MIN, Math.floor(root.clientWidth * 0.6)) : 480;
    const next = drag.current.startW + (e.clientX - drag.current.startX);
    setTreeWidth(Math.min(max, Math.max(TREE_MIN, next)));
  }, []);

  const onSplitPointerUp = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    drag.current = null;
    if (e.currentTarget.hasPointerCapture(e.pointerId)) {
      e.currentTarget.releasePointerCapture(e.pointerId);
    }
  }, []);

  return (
    <>
      <div className="explorer-tree" style={{ flexBasis: treeWidth, width: treeWidth }}>
        <FolderTree
          repoName={repoName}
          snapshotId={snapshotId}
          currentPath={b.path}
          onNavigate={b.navigateTo}
        />
      </div>
      <div
        className="explorer-split"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize tree and listing"
        aria-valuenow={treeWidth}
        tabIndex={0}
        onPointerDown={onSplitPointerDown}
        onPointerMove={onSplitPointerMove}
        onPointerUp={onSplitPointerUp}
        onPointerCancel={onSplitPointerUp}
      />
      <ExplorerListing
        repoName={repoName}
        snapshotId={snapshotId}
        snapshotLabel={snapshotLabel}
        path={b.path}
        entries={b.entries}
        selection={b.selection}
        listingReady={b.listingReady}
        loading={b.loading}
        carryReady={b.carryReady}
        carryNote={b.carryNote}
        carryError={b.carryError}
        carryRequested={b.carryRequested}
        loadError={b.loadError}
        actionError={b.actionError}
        actionBusy={actionBusy}
        onActionId={onActionId}
        onBusyLabel={onBusyLabel}
        onActionError={b.setActionError}
        onSelect={(e) =>
          b.setSelection({ path: e.path, name: e.name, is_dir: e.is_dir })
        }
        onNavigate={b.navigateTo}
        onRetryListing={b.retryListing}
        onRetryCarry={b.retryCarry}
      />
    </>
  );
}
