import { useEffect, useRef, useState } from "react";
import { FileActions, type FileSelection } from "./FileActions";
import { FileListing } from "./FileListing";
import { FolderTree } from "./FolderTree";
import {
  clearListDirCacheForRepo,
  loadListDir,
  resolveCarryPath,
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
  const [carryNote, setCarryNote] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [carryReady, setCarryReady] = useState(true);
  const [loadedPath, setLoadedPath] = useState<string | null>(null);
  const [loadedIdentity, setLoadedIdentity] = useState<string | null>(null);
  const prevRepo = useRef(repoName);
  const prevSnap = useRef(snapshotId);
  const pathRef = useRef(path);
  pathRef.current = path;

  useEffect(() => {
    const repoChanged = prevRepo.current !== repoName;
    const snapChanged = prevSnap.current !== snapshotId;
    if (repoChanged) {
      clearListDirCacheForRepo(prevRepo.current);
      prevRepo.current = repoName;
      prevSnap.current = snapshotId;
      setPath("/");
      setCarryNote(null);
      setSelection(null);
      setEntries([]);
      setLoadError(null);
      setActionError(null);
      setLoading(true);
      setLoadedPath(null);
      setLoadedIdentity(null);
      setCarryReady(true);
      return;
    }
    if (!snapChanged) return;
    prevSnap.current = snapshotId;
    const requested = pathRef.current;
    setSelection(null);
    setActionError(null);
    setCarryNote(null);
    setCarryReady(false);
    setLoading(true);
    setEntries([]);
    setLoadedPath(null);
    setLoadedIdentity(null);
    let discarded = false;
    resolveCarryPath(repoName, snapshotId, requested)
      .then((r) => {
        if (discarded) return;
        setPath(r.path);
        setCarryNote(r.note);
        setCarryReady(true);
      })
      .catch((e: unknown) => {
        if (discarded) return;
        setPath("/");
        setCarryNote(String(e));
        setCarryReady(true);
      });
    return () => {
      discarded = true;
    };
  }, [repoName, snapshotId]);

  useEffect(() => {
    if (!carryReady) return;
    let discarded = false;
    setLoading(true);
    setLoadedPath(null);
    setLoadedIdentity(null);
    setLoadError(null);
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
  }, [repoName, snapshotId, path, activeIdentity, carryReady]);

  function navigateTo(next: string) {
    setCarryNote(null);
    setLoading(true);
    setLoadedPath(null);
    setLoadedIdentity(null);
    setPath(next);
  }

  function selectEntry(entry: DirEntry) {
    setSelection({ path: entry.path, name: entry.name, is_dir: entry.is_dir });
  }

  const listingReady =
    carryReady &&
    !loading &&
    loadedPath === path &&
    loadedIdentity === activeIdentity &&
    !loadError;

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
            {carryNote && <p className="app-loading">{carryNote}</p>}
            {loadError && (
              <div>
                <pre className="app-error">{loadError}</pre>
                <button type="button" onClick={() => navigateTo(path)}>
                  Retry
                </button>
              </div>
            )}
            {actionError && <pre className="app-error">{actionError}</pre>}
            {(loading || !carryReady) && <p className="app-loading">Loading directory…</p>}
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
