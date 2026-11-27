import { useCallback, useRef, useState, type PointerEvent as ReactPointerEvent } from "react";
import { ExplorerListing } from "./ExplorerListing";
import { FolderTree } from "./FolderTree";
import type { DirEntry } from "./listDirCache";
import { useExplorerBrowse } from "./useExplorerBrowse";
import type { ShowHitRequest } from "./usePendingShowHit";

export type { DirEntry };
export type { ShowHitRequest };

const TREE_MIN = 128;
const TREE_DEFAULT = 224;

type Props = {
  repoName: string;
  snapshotId: string;
  snapshotLabel: string;
  actionBusy?: boolean;
  overwriteOpen?: boolean;
  showHitRequest?: ShowHitRequest | null;
  onShowHitConsumed?: () => void;
  onActionId?: (id: string | null) => void;
  onBusyLabel?: (label: string | null) => void;
  onOverwriteOpenChange?: (open: boolean) => void;
};

export function ExplorerView({
  repoName,
  snapshotId,
  snapshotLabel,
  actionBusy = false,
  overwriteOpen = false,
  showHitRequest = null,
  onShowHitConsumed,
  onActionId,
  onBusyLabel,
  onOverwriteOpenChange,
}: Props) {
  const b = useExplorerBrowse(
    repoName,
    snapshotId,
    showHitRequest,
    onShowHitConsumed,
  );
  const [treeWidth, setTreeWidth] = useState(TREE_DEFAULT);
  const [treeCollapsed, setTreeCollapsed] = useState(false);
  const drag = useRef<{ startX: number; startW: number } | null>(null);
  const panesRef = useRef<HTMLDivElement>(null);

  const onSplitPointerDown = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    e.preventDefault();
    drag.current = { startX: e.clientX, startW: treeWidth };
    e.currentTarget.setPointerCapture(e.pointerId);
  }, [treeWidth]);

  const onSplitPointerMove = useCallback((e: ReactPointerEvent<HTMLDivElement>) => {
    if (!drag.current) return;
    const root = panesRef.current;
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

  const statusSelection = b.selection
    ? `${b.selection.name} selected`
    : "No selection";

  return (
    <div className="explorer-shell">
      <div className="explorer-panes" ref={panesRef}>
        {!treeCollapsed && (
          <div
            className="explorer-tree"
            style={{ flexBasis: treeWidth, width: treeWidth }}
          >
            <div className="explorer-tree-header">
              <span className="explorer-tree-title">Folders</span>
              <button
                type="button"
                className="explorer-tree-toggle"
                aria-expanded={true}
                onClick={() => setTreeCollapsed(true)}
              >
                Hide
              </button>
            </div>
            <FolderTree
              repoName={repoName}
              snapshotId={snapshotId}
              currentPath={b.path}
              onNavigate={b.navigateTo}
            />
          </div>
        )}
        {!treeCollapsed && (
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
        )}
        {treeCollapsed && (
          <button
            type="button"
            className="explorer-tree-show"
            aria-expanded={false}
            onClick={() => setTreeCollapsed(false)}
          >
            Show folders
          </button>
        )}
        <ExplorerListing
          repoName={repoName}
          snapshotId={snapshotId}
          snapshotLabel={snapshotLabel}
          path={b.path}
          entries={b.entries}
          selection={b.selection}
          listingReady={b.listingReady}
          showStaleListing={b.showStaleListing}
          loading={b.loading}
          carryReady={b.carryReady}
          carryNote={b.carryNote}
          carryError={b.carryError}
          carryRequested={b.carryRequested}
          loadError={b.loadError}
          actionError={b.actionError}
          actionBusy={actionBusy}
          overwriteOpen={overwriteOpen}
          onActionId={onActionId}
          onBusyLabel={onBusyLabel}
          onOverwriteOpenChange={onOverwriteOpenChange}
          onActionError={b.setActionError}
          onSelect={(e) =>
            b.setSelection({ path: e.path, name: e.name, is_dir: e.is_dir })
          }
          onNavigate={b.navigateTo}
          onRetryListing={b.retryListing}
          onRetryCarry={b.retryCarry}
        />
      </div>
      {b.listingReady && (
        <footer className="file-status-bar" aria-live="polite">
          {b.entries.length} entries · {statusSelection}
        </footer>
      )}
    </div>
  );
}
