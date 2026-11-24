import { useEffect, useState } from "react";
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
  const trimmed = path.replace(/\/+$/, "");
  const idx = trimmed.lastIndexOf("/");
  if (idx <= 0) return "/";
  return trimmed.slice(0, idx);
}

export function FileBrowser({ repoName, snapshotId }: Props) {
  const [path, setPath] = useState("/");
  const [entries, setEntries] = useState<DirEntry[]>([]);
  const [selection, setSelection] = useState<Selection>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    setPath("/");
    setEntries([]);
    setSelection(null);
    setError(null);
  }, [repoName, snapshotId]);

  useEffect(() => {
    let discarded = false;
    setLoading(true);
    setError(null);
    setEntries([]);
    setSelection(null);
    invoke<DirEntry[]>("list_dir", {
      repoName,
      snapshotId,
      path,
    })
      .then((rows) => {
        if (!discarded) setEntries(rows);
      })
      .catch((e: unknown) => {
        if (!discarded) {
          setEntries([]);
          setError(String(e));
        }
      })
      .finally(() => {
        if (!discarded) setLoading(false);
      });
    return () => {
      discarded = true;
    };
  }, [repoName, snapshotId, path]);

  function onRowClick(entry: DirEntry) {
    if (entry.is_dir) {
      setPath(entry.path);
      return;
    }
    setSelection({ path: entry.path, is_dir: false });
  }

  const atRoot = path === "/";

  return (
    <section className="file-browser">
      <div className="file-browser-toolbar">
        <button type="button" disabled={atRoot || loading} onClick={() => setPath(parentPath(path))}>
          Up
        </button>
        <span className="mono file-browser-path">{path}</span>
      </div>
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
                onClick={() => onRowClick(e)}
                style={{ cursor: "pointer" }}
              >
                <td className="mono">{e.name}</td>
                <td>{e.is_dir ? "dir" : "file"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  );
}
