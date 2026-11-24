import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

export type DirEntry = {
  name: string;
  path: string;
  is_dir: boolean;
  size?: number | null;
};

type Selection = { path: string; is_dir: boolean } | null;

type Props = {
  repoName: string;
  snapshotId: string;
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

export function FileBrowser({ repoName, snapshotId }: Props) {
  const [path, setPath] = useState("/");
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [selection, setSelection] = useState<Selection>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  /** Path whose listing is currently shown (set only when that request finishes). */
  const [loadedPath, setLoadedPath] = useState<string | null>(null);
  const upRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  /** Path we still owe focus to, once its listing has finished rendering. */
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
    invoke<DirEntry[]>("list_dir", {
      repoName,
      snapshotId,
      path: requestedPath,
    })
      .then((rows) => {
        if (discarded) return;
        setEntries(rows);
        setLoadedPath(requestedPath);
      })
      .catch((e: unknown) => {
        if (discarded) return;
        setEntries([]);
        setError(String(e));
        setLoadedPath(requestedPath);
      })
      .finally(() => {
        if (!discarded) setLoading(false);
      });
    return () => {
      discarded = true;
    };
  }, [repoName, snapshotId, path]);

  useEffect(() => {
    if (loading) return;
    if (loadedPath !== path) return;
    if (pendingFocusPath.current !== path) return;
    pendingFocusPath.current = null;

    const atRoot = path === "/";
    if (!atRoot && upRef.current && !upRef.current.disabled) {
      upRef.current.focus();
      return;
    }
    const first = listRef.current?.querySelector<HTMLButtonElement>("button.linkish");
    if (first) {
      first.focus();
      return;
    }
    upRef.current?.focus();
  }, [loading, loadedPath, path, entries, error]);

  function navigateTo(next: string) {
    pendingFocusPath.current = next;
    setLoading(true);
    setLoadedPath(null);
    setPath(next);
  }

  function openEntry(entry: DirEntry) {
    if (entry.is_dir) {
      navigateTo(entry.path);
      return;
    }
    setSelection({ path: entry.path, is_dir: false });
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
              {entries.map((e) => {
                const selected = selection?.path === e.path;
                return (
                  <tr key={e.path} className={selected ? "is-selected" : undefined}>
                    <td className="mono">
                      <button
                        type="button"
                        className="linkish"
                        aria-current={selected ? "true" : undefined}
                        onClick={() => openEntry(e)}
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
