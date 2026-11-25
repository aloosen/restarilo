import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { FileBrowser } from "./FileBrowser";
import { clearListDirCacheForRepo } from "./listDirCache";
import {
  SnapshotList,
  sortSnapshotsNewestFirst,
  type SnapshotRow,
} from "./SnapshotList";

type RepoName = { name: string };

function App() {
  const [repos, setRepos] = useState<RepoName[]>([]);
  const [selectedRepo, setSelectedRepo] = useState("");
  const [snapshots, setSnapshots] = useState<SnapshotRow[]>([]);
  const [selectedSnapshot, setSelectedSnapshot] = useState<SnapshotRow | null>(
    null,
  );
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [actionId, setActionId] = useState<string | null>(null);
  const actionBusy = actionId !== null;

  useEffect(() => {
    invoke<RepoName[]>("list_repos")
      .then((list) => {
        setRepos(list);
        if (list[0]) setSelectedRepo(list[0].name);
      })
      .catch((e: unknown) => setError(String(e)));
  }, []);

  useEffect(() => {
    if (!selectedRepo) return;
    let discarded = false;
    setSelectedSnapshot(null);
    setSnapshots([]);
    setLoading(true);
    setError(null);
    invoke<SnapshotRow[]>("list_snapshots", { repoName: selectedRepo })
      .then((rows) => {
        if (discarded) return;
        const sorted = sortSnapshotsNewestFirst(rows);
        setSnapshots(sorted);
        setSelectedSnapshot(sorted[0] ?? null);
      })
      .catch((e: unknown) => {
        if (discarded) return;
        setSnapshots([]);
        setSelectedSnapshot(null);
        setError(String(e));
      })
      .finally(() => {
        if (!discarded) setLoading(false);
      });
    return () => {
      discarded = true;
      clearListDirCacheForRepo(selectedRepo);
    };
  }, [selectedRepo]);

  const onCancel = useCallback(() => {
    if (!actionId) return;
    void invoke("cancel_action", { actionId });
  }, [actionId]);

  useEffect(() => {
    if (!actionBusy) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [actionBusy, onCancel]);

  function onSelectRepo(name: string) {
    if (name === selectedRepo) return;
    clearListDirCacheForRepo(selectedRepo);
    setSelectedSnapshot(null);
    setSnapshots([]);
    setError(null);
    setLoading(true);
    setSelectedRepo(name);
  }

  function onSelectSnapshotId(id: string) {
    const row = snapshots.find((s) => s.id === id) ?? null;
    setSelectedSnapshot(row);
  }

  const emptyExplorer = !loading && snapshots.length === 0;
  const showExplorer = selectedSnapshot !== null && !loading;

  return (
    <main className="app">
      <h1>restarilo</h1>

      <div className="explorer-chrome">
        <label className="app-repo">
          Repo{" "}
          <select
            value={selectedRepo}
            onChange={(e) => onSelectRepo(e.target.value)}
            disabled={repos.length === 0 || actionBusy}
          >
            {repos.map((r) => (
              <option key={r.name} value={r.name}>
                {r.name}
              </option>
            ))}
          </select>
        </label>
        <SnapshotList
          snapshots={snapshots}
          selectedId={selectedSnapshot?.id ?? null}
          loading={loading}
          disabled={actionBusy}
          error={error}
          onSelectId={onSelectSnapshotId}
        />
      </div>

      {actionBusy && (
        <div className="app-action-bar">
          <span className="file-actions-busy">Working…</span>
          <button type="button" onClick={onCancel}>
            Cancel
          </button>
        </div>
      )}

      <section className="explorer">
        {showExplorer && selectedSnapshot ? (
          <FileBrowser
            repoName={selectedRepo}
            snapshotId={selectedSnapshot.id}
            snapshotLabel={
              selectedSnapshot.short_id || selectedSnapshot.id.slice(0, 8)
            }
            actionBusy={actionBusy}
            onActionId={setActionId}
          />
        ) : (
          <div className="explorer-listing">
            {emptyExplorer ? (
              <p className="app-loading">No snapshots in this repository.</p>
            ) : (
              <p className="app-loading">Loading snapshots…</p>
            )}
          </div>
        )}
      </section>
    </main>
  );
}

export default App;
