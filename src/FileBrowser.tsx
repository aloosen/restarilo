import { useEffect, useRef, useState, type KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FileActions, type FileSelection } from "./FileActions";

export type DirEntry = {
  name: string;
  path: string;
  is_dir: boolean;
  size?: number | null;
};

type Props = { repoName: string; snapshotId: string };

function parentPath(path: string): string {
  if (path === "/" || path === "") return "/";
  let end = path.length;
  while (end > 1 && path.charAt(end - 1) === "/") end -= 1;
  const trimmed = path.slice(0, end);
  const idx = trimmed.lastIndexOf("/");
  if (idx <= 0) return "/";
  return trimmed.slice(0, idx);
}

export function FileBrowser({ repoName, snapshotId }: Props) {
  const [path, setPath] = useState("/");
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [selection, setSelection] = useState<FileSelection | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadedPath, setLoadedPath] = useState<string | null>(null);
  const upRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const pendingFocusPath = useRef<string | null>("/");

  useEffect(() => {
    setPath("/");
    setEntries([]);
    setSelection(null);
    setError(null);
    setLoading(true);
    setLoadedPath(null);
    pendingFocusPath.current = "/";
  }, [repoName, snapshotId]);

  useEffect(() => {
    let discarded = false;
    setLoading(true);
    setLoadedPath(null);
    setError(null);
    setEntries([]);
    setSelection(null);
    const requestedPath = path;
    invoke<DirEntry[]>("list_dir", { repoName, snapshotId, path: requestedPath })
      .then((rows) => {
        if (!discarded) {
          setEntries(rows);
          setLoadedPath(requestedPath);
        }
      })
      .catch((e: unknown) => {
        if (!discarded) {
          setEntries([]);
          setError(String(e));
          setLoadedPath(requestedPath);
        }
      })
      .finally(() => {
        if (!discarded) setLoading(false);
      });
    return () => {
      discarded = true;
    };
  }, [repoName, snapshotId, path]);

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
  }, [loading, loadedPath, path, entries, error]);

  function navigateTo(next: string) {
    pendingFocusPath.current = next;
    setLoading(true);
    setLoadedPath(null);
    setPath(next);
  }

  function selectEntry(entry: DirEntry) {
    setSelection({ path: entry.path, name: entry.name, is_dir: entry.is_dir });
  }

  function onEntryKeyDown(e: KeyboardEvent, entry: DirEntry) {
    if (e.key === "Enter" && entry.is_dir) {
      e.preventDefault();
      navigateTo(entry.path);
    }
  }

  const atRoot = path === "/";

  return (
    <section className="file-browser">
      <div className="file-browser-toolbar">
        <button
          ref={upRef}
          type="button"
          disabled={atRoot || loading}
          onClick={() => navigateTo(parentPath(path))}
        >
          Up
        </button>
        <span className="mono file-browser-path">{path}</span>
        <FileActions
          repoName={repoName}
          snapshotId={snapshotId}
          selection={selection}
          disabled={loading}
          onError={setError}
        />
      </div>
      <div className="file-browser-body" ref={listRef}>
        {error && <pre className="app-error">{error}</pre>}
        {loading && <p className="app-loading">Loading directory…</p>}
        {!loading && !error && (
          <table className="app-table">
            <thead>
              <tr>
                <th>Name</th>
                <th>Type</th>
              </tr>
            </thead>
            <tbody>
              {entries.map((e) => (
                <tr
                  key={e.path}
                  className={selection?.path === e.path ? "is-selected" : undefined}
                >
                  <td className="mono">
                    <button
                      type="button"
                      className="linkish"
                      aria-current={selection?.path === e.path ? "true" : undefined}
                      onClick={() => selectEntry(e)}
                      onDoubleClick={() => e.is_dir && navigateTo(e.path)}
                      onKeyDown={(ev) => onEntryKeyDown(ev, e)}
                    >
                      {e.name}
                    </button>
                  </td>
                  <td>{e.is_dir ? "dir" : "file"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </section>
  );
}
