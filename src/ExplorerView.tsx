import { useEffect, useRef, useState } from "react";
import { FileActions, type FileSelection } from "./FileActions";
import { FileListing } from "./FileListing";
import { FolderTree } from "./FolderTree";
import {
  clearListDirCacheForRepo,
  loadListDir,
  type DirEntry,
} from "./listDirCache";

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
  const activeIdentity = `${repoName}\0${snapshotId}`;
  const [path, setPath] = useState("/");
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [selection, setSelection] = useState<FileSelection | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadedPath, setLoadedPath] = useState<string | null>(null);
  const [loadedIdentity, setLoadedIdentity] = useState<string | null>(null);
  const prevRepo = useRef(repoName);

  useEffect(() => {
    if (prevRepo.current !== repoName) {
      clearListDirCacheForRepo(prevRepo.current);
      prevRepo.current = repoName;
    }
    setPath("/");
    setEntries([]);
    setSelection(null);
    setLoadError(null);
    setActionError(null);
    setLoading(true);
    setLoadedPath(null);
    setLoadedIdentity(null);
  }, [repoName, snapshotId]);

  useEffect(() => {
    let discarded = false;
    setLoading(true);
    setLoadedPath(null);
    setLoadedIdentity(null);
    setLoadError(null);
    setActionError(null);
    setEntries([]);
    setSelection(null);
    const requestedPath = path;
    const requestedIdentity = activeIdentity;
    loadListDir(repoName, snapshotId, requestedPath)
      .then((rows) => {
        if (discarded) return;
        setEntries(rows);
        setLoadedPath(requestedPath);
        setLoadedIdentity(requestedIdentity);
      })
      .catch((e: unknown) => {
        if (discarded) return;
        setEntries([]);
        setLoadError(String(e));
        setLoadedPath(requestedPath);
        setLoadedIdentity(requestedIdentity);
      })
      .finally(() => {
        if (!discarded) setLoading(false);
      });
    return () => {
      discarded = true;
    };
  }, [repoName, snapshotId, path, activeIdentity]);

  function navigateTo(next: string) {
    setLoading(true);
    setLoadedPath(null);
    setLoadedIdentity(null);
    setPath(next);
  }

  function selectEntry(entry: DirEntry) {
    setSelection({ path: entry.path, name: entry.name, is_dir: entry.is_dir });
  }

  const listingReady =
    !loading && loadedPath === path && loadedIdentity === activeIdentity && !loadError;

  return (
    <>
      <div className="explorer-tree">
        <FolderTree
          repoName={repoName}
          snapshotId={snapshotId}
          currentPath={path}
          onNavigate={navigateTo}
        />
      </div>
      <div className="explorer-listing">
        <section className="file-browser">
          <div className="file-browser-toolbar explorer-chrome-paths">
            <span className="mono">
              {repoName} · {snapshotLabel} · {path}
              {selection ? ` · sel ${selection.path}` : ""}
            </span>
            <FileActions
              repoName={repoName}
              snapshotId={snapshotId}
              selection={listingReady ? selection : null}
              disabled={!listingReady || actionBusy}
              actionBusy={actionBusy}
              onActionId={onActionId}
              onError={setActionError}
            />
          </div>
          <div className="file-browser-body">
            {loadError && (
              <div>
                <pre className="app-error">{loadError}</pre>
                <button type="button" onClick={() => navigateTo(path)}>
                  Retry
                </button>
              </div>
            )}
            {actionError && <pre className="app-error">{actionError}</pre>}
            {loading && <p className="app-loading">Loading directory…</p>}
            {listingReady && (
              <FileListing
                path={path}
                entries={entries}
                selectedPath={selection?.path ?? null}
                onSelect={selectEntry}
                onNavigate={navigateTo}
              />
            )}
          </div>
        </section>
      </div>
    </>
  );
}
