import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { FileBrowser } from "./FileBrowser";
import { formatSnapshotLabel } from "./formatSize";
import { clearListDirCacheForRepo } from "./listDirCache";
import {
  SnapshotList,
  sortSnapshotsNewestFirst,
  type SnapshotRow,
} from "./SnapshotList";

type RepoName = { name: string };

function ArchiveMark() {
  return (
    <span className="app-brand-mark" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
        <rect x="2" y="4" width="20" height="16" rx="2" />
        <path d="M2 10h20" />
        <path d="M12 4v16" />
      </svg>
    </span>
  );
}

function App() {
  const [repos, setRepos] = useState<RepoName[]>([]);
  const [reposReady, setReposReady] = useState(false);
  const [configError, setConfigError] = useState<string | null>(null);
  const [selectedRepo, setSelectedRepo] = useState("");
  const [snapshots, setSnapshots] = useState<SnapshotRow[]>([]);
  const [selectedSnapshot, setSelectedSnapshot] = useState<SnapshotRow | null>(null);
  const [snapError, setSnapError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [snapReload, setSnapReload] = useState(0);
  const [actionId, setActionId] = useState<string | null>(null);
  const [busyLabel, setBusyLabel] = useState<string | null>(null);
  const actionBusy = actionId !== null;

  useEffect(() => {
    invoke<RepoName[]>("list_repos")
      .then((list) => {
        setRepos(list);
        setReposReady(true);
        if (list[0]) setSelectedRepo(list[0].name);
      })
      .catch((e: unknown) => {
        setConfigError(String(e));
        setReposReady(true);
      });
  }, []);

  useEffect(() => {
    if (!selectedRepo) return;
    let discarded = false;
    setSelectedSnapshot(null);
    setSnapshots([]);
    setLoading(true);
    setSnapError(null);
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
        setSnapError(String(e));
      })
      .finally(() => {
        if (!discarded) setLoading(false);
      });
    return () => {
      discarded = true;
      clearListDirCacheForRepo(selectedRepo);
    };
  }, [selectedRepo, snapReload]);

  const onCancel = useCallback(() => {
    if (actionId) void invoke("cancel_action", { actionId });
  }, [actionId]);

  useEffect(() => {
    if (!actionBusy) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [actionBusy, onCancel]);

  function onSelectRepo(name: string) {
    if (name === selectedRepo) return;
    clearListDirCacheForRepo(selectedRepo);
    setSelectedSnapshot(null);
    setSnapshots([]);
    setSnapError(null);
    setLoading(true);
    setSelectedRepo(name);
  }

  const emptyRepo =
    !loading && !snapError && selectedRepo !== "" && snapshots.length === 0;
  const showExplorer = selectedSnapshot !== null && !loading && !snapError;
  const zeroRepos = reposReady && !configError && repos.length === 0;

  return (
    <main className="app">
      <header className="explorer-chrome">
        <div className="app-brand">
          <ArchiveMark />
          <span className="app-brand-name">restarilo</span>
        </div>
        <div className="app-chrome-controls">
          {configError ? (
            <p className="app-error app-config-error">
              Could not load configuration. Fix config.toml and restart restarilo.
              {"\n"}
              {configError}
            </p>
          ) : zeroRepos ? (
            <p className="app-loading">
              No repositories configured. Add [[repos]] entries to config.toml and
              restart restarilo.
            </p>
          ) : (
            <>
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
                listError={snapError}
                onSelectId={(id) =>
                  setSelectedSnapshot(snapshots.find((s) => s.id === id) ?? null)
                }
                onRetryList={() => selectedRepo && setSnapReload((n) => n + 1)}
              />
            </>
          )}
        </div>
      </header>

      {actionBusy && (
        <div className="app-action-bar">
          <span className="file-actions-busy">{busyLabel ?? "Working…"}</span>
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
            snapshotLabel={formatSnapshotLabel(selectedSnapshot)}
            actionBusy={actionBusy}
            onActionId={setActionId}
            onBusyLabel={setBusyLabel}
          />
        ) : (
          <div className="explorer-listing">
            {configError || zeroRepos ? (
              <p className="app-loading">Configure a repository to browse snapshots.</p>
            ) : snapError ? (
              <p className="app-loading">
                Could not load snapshots. Use Retry next to the snapshot control.
              </p>
            ) : emptyRepo ? (
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
