import { ExplorerListing } from "./ExplorerListing";
import { FolderTree } from "./FolderTree";
import type { DirEntry } from "./listDirCache";
import { useExplorerBrowse } from "./useExplorerBrowse";

export type { DirEntry };

type Props = {
  repoName: string;
  snapshotId: string;
  snapshotLabel: string;
  actionBusy?: boolean;
  onActionId?: (id: string | null) => void;
};

export function ExplorerView({
  repoName,
  snapshotId,
  snapshotLabel,
  actionBusy = false,
  onActionId,
}: Props) {
  const b = useExplorerBrowse(repoName, snapshotId);

  return (
    <>
      <div className="explorer-tree">
        <FolderTree
          repoName={repoName}
          snapshotId={snapshotId}
          currentPath={b.path}
          onNavigate={b.navigateTo}
        />
      </div>
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
