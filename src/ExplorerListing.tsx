import { FileActions, type FileSelection } from "./FileActions";
import { FileListing } from "./FileListing";
import type { DirEntry } from "./listDirCache";

type Props = {
  repoName: string;
  snapshotId: string;
  snapshotLabel: string;
  path: string;
  entries: DirEntry[];
  selection: FileSelection | null;
  listingReady: boolean;
  loading: boolean;
  carryReady: boolean;
  carryNote: string | null;
  carryError: string | null;
  carryRequested: string | null;
  loadError: string | null;
  actionError: string | null;
  actionBusy: boolean;
  onActionId?: (id: string | null) => void;
  onActionError: (message: string | null) => void;
  onSelect: (entry: DirEntry) => void;
  onNavigate: (path: string) => void;
  onRetryListing: () => void;
  onRetryCarry: () => void;
};

export function ExplorerListing(p: Props) {
  return (
    <div className="explorer-listing">
      <section className="file-browser">
        <div className="file-browser-toolbar explorer-chrome-paths">
          <span className="mono">
            {p.repoName} · {p.snapshotLabel} · {p.path}
            {p.selection ? ` · sel ${p.selection.path}` : ""}
          </span>
          <FileActions
            repoName={p.repoName}
            snapshotId={p.snapshotId}
            selection={p.listingReady ? p.selection : null}
            disabled={!p.listingReady || p.actionBusy}
            actionBusy={p.actionBusy}
            onActionId={p.onActionId}
            onError={p.onActionError}
          />
        </div>
        <div className="file-browser-body">
          {p.carryNote && <p className="app-loading">{p.carryNote}</p>}
          {p.carryError && (
            <div>
              <pre className="app-error">{p.carryError}</pre>
              <p className="app-loading">
                Could not open {p.carryRequested ?? p.path}; showing {p.path}.
              </p>
              <button type="button" onClick={p.onRetryCarry}>
                Retry {p.carryRequested ?? p.path}
              </button>
            </div>
          )}
          {p.loadError && (
            <div>
              <pre className="app-error">{p.loadError}</pre>
              <button type="button" onClick={p.onRetryListing}>
                Retry
              </button>
            </div>
          )}
          {p.actionError && <pre className="app-error">{p.actionError}</pre>}
          {(p.loading || !p.carryReady) && !p.carryError && (
            <p className="app-loading">Loading directory…</p>
          )}
          {p.listingReady && (
            <FileListing
              path={p.path}
              entries={p.entries}
              selectedPath={p.selection?.path ?? null}
              onSelect={p.onSelect}
              onNavigate={p.onNavigate}
            />
          )}
        </div>
      </section>
    </div>
  );
}
