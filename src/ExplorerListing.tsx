import { useEffect, useState } from "react";
import { FileActions, type FileSelection } from "./FileActions";
import {
  FileListing,
  breadcrumbParts,
  folderHeading,
} from "./FileListing";
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
  onBusyLabel?: (label: string | null) => void;
  onActionError: (message: string | null) => void;
  onSelect: (entry: DirEntry) => void;
  onNavigate: (path: string) => void;
  onRetryListing: () => void;
  onRetryCarry: () => void;
};

export function ExplorerListing(p: Props) {
  const [success, setSuccess] = useState<string | null>(null);
  const crumbs = breadcrumbParts(p.path);
  const heading = folderHeading(p.path);

  useEffect(() => {
    setSuccess(null);
    p.onActionError(null);
  }, [p.repoName, p.snapshotId]);

  useEffect(() => {
    setSuccess(null);
    p.onActionError(null);
  }, [p.selection?.path]);

  function onSelect(entry: DirEntry) {
    setSuccess(null);
    p.onActionError(null);
    p.onSelect(entry);
  }

  return (
    <div className="explorer-listing">
      <section className="file-browser">
        <nav className="file-breadcrumb" aria-label="Path">
          {crumbs.map((c, i) => (
            <span key={c.target}>
              {i > 0 && <span className="file-breadcrumb-sep"> / </span>}
              {c.target === p.path ? (
                <span aria-current="page">{c.label}</span>
              ) : (
                <button
                  type="button"
                  className="linkish file-breadcrumb-btn"
                  onClick={() => p.onNavigate(c.target)}
                >
                  {c.label}
                </button>
              )}
            </span>
          ))}
        </nav>
        <div className="file-browser-heading">
          <h2 className="file-folder-title">{heading}</h2>
          <FileActions
            repoName={p.repoName}
            snapshotId={p.snapshotId}
            snapshotLabel={p.snapshotLabel}
            selection={p.listingReady ? p.selection : null}
            disabled={!p.listingReady || p.actionBusy}
            actionBusy={p.actionBusy}
            onActionId={p.onActionId}
            onBusyLabel={p.onBusyLabel}
            onSuccess={setSuccess}
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
          {success && <p className="app-success">{success}</p>}
          {p.actionError && <pre className="app-error">{p.actionError}</pre>}
          {(p.loading || !p.carryReady) && !p.carryError && (
            <p className="app-loading">Loading directory…</p>
          )}
          {p.listingReady && p.entries.length === 0 && (
            <p className="app-loading">This folder is empty.</p>
          )}
          {p.listingReady && p.entries.length > 0 && (
            <FileListing
              path={p.path}
              entries={p.entries}
              selectedPath={p.selection?.path ?? null}
              onSelect={onSelect}
              onNavigate={p.onNavigate}
            />
          )}
        </div>
      </section>
    </div>
  );
}
