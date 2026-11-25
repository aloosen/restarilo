import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FileActions, type FileSelection } from "./FileActions";

export type DirEntry = {
  name: string;
  path: string;
  is_dir: boolean;
  size?: number | null;
};

type Props = {
  repoName: string;
  snapshotId: string;
  actionBusy?: boolean;
  onActionId?: (id: string | null) => void;
};

function parentPath(path: string): string {
  if (path === "/" || path === "") return "/";
  let end = path.length;
  while (end > 1 && path.charAt(end - 1) === "/") end -= 1;
  const trimmed = path.slice(0, end);
  const idx = trimmed.lastIndexOf("/");
  if (idx <= 0) return "/";
  return trimmed.slice(0, idx);
}

export function FileBrowser({
  repoName,
  snapshotId,
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
  const upRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const pendingFocusPath = useRef<string | null>("/");

  function resetListing() {
    setEntries([]);
    setSelection(null);
    setLoadError(null);
    setActionError(null);
    setLoading(true);
    setLoadedPath(null);
    setLoadedIdentity(null);
  }

  useEffect(() => {
    setPath("/");
    resetListing();
    pendingFocusPath.current = "/";
  }, [repoName, snapshotId]);

  useEffect(() => {
    let discarded = false;
    resetListing();
    const requestedPath = path;
    const requestedIdentity = activeIdentity;
    invoke<DirEntry[]>("list_dir", {
      repoName,
      snapshotId,
      path: requestedPath,
    })
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

  useEffect(() => {
    if (loading || loadedPath !== path || pendingFocusPath.current !== path) return;
    pendingFocusPath.current = null;
    if (path !== "/" && upRef.current && !upRef.current.disabled) {
      upRef.current.focus();
      return;
    }
    const first = listRef.current?.querySelector<HTMLButtonElement>("button.linkish");
    if (first) first.focus();
    else upRef.current?.focus();
  }, [loading, loadedPath, path, entries, loadError]);

  function navigateTo(next: string) {
    pendingFocusPath.current = next;
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
    <section className="file-browser">
      <div className="file-browser-toolbar">
        <button
          ref={upRef}
          type="button"
          disabled={path === "/" || loading}
          onClick={() => navigateTo(parentPath(path))}
        >
          Up
        </button>
        <span className="mono file-browser-path">{path}</span>
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
      <div className="file-browser-body" ref={listRef}>
        {loadError && <pre className="app-error">{loadError}</pre>}
        {actionError && <pre className="app-error">{actionError}</pre>}
        {loading && <p className="app-loading">Loading directory…</p>}
        {listingReady && (
          <table className="app-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Type</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((e) => {
                const selected = selection?.path === e.path;
                return (
                  <tr
                    key={e.path}
                    className={
                      selected ? "is-selectable is-selected" : "is-selectable"
                    }
                    onClick={() => selectEntry(e)}
                    onDoubleClick={() => e.is_dir && navigateTo(e.path)}
                  >
                    <td className="mono">
                      <button
                        type="button"
                        className="linkish"
                        aria-current={selected ? "true" : undefined}
                        onClick={(event) => {
                          if (!e.is_dir) return;
                          event.stopPropagation();
                          navigateTo(e.path);
                        }}
                        onKeyDown={(ev: KeyboardEvent) => {
                          if (ev.key !== " ") return;
                          ev.preventDefault();
                          selectEntry(e);
                        }}
                      >
                        {e.name}
                      </button>
                    </td>
                    <td>{e.is_dir ? "dir" : "file"}</td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        )}
      </div>
    </section>
  );
}
