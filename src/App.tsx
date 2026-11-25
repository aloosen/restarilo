import { useEffect, useRef, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { FileBrowser } from "./FileBrowser";
import { SnapshotList, type SnapshotRow } from "./SnapshotList";

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
  const filesPaneRef = useRef<HTMLElement>(null);
  const snapshotsPaneRef = useRef<HTMLElement>(null);
  const returnFocusId = useRef<string | null>(null);
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
    setLoading(true);
    setError(null);
    invoke<SnapshotRow[]>("list_snapshots", { repoName: selectedRepo })
      .then((rows) => {
        if (!discarded) setSnapshots(rows);
      })
      .catch((e: unknown) => {
        if (!discarded) {
          setSnapshots([]);
          setError(String(e));
        }
      })
      .finally(() => {
        if (!discarded) setLoading(false);
      });
    return () => {
      discarded = true;
    };
  }, [selectedRepo]);

  useEffect(() => {
    if (!selectedSnapshot) return;
    filesPaneRef.current?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, [selectedSnapshot]);

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

  function selectSnapshot(row: SnapshotRow) {
    returnFocusId.current = row.id;
    setSelectedSnapshot(row);
  }

  function backToSnapshots() {
    const id = returnFocusId.current ?? selectedSnapshot?.id ?? null;
    setSelectedSnapshot(null);
    requestAnimationFrame(() => {
      const root = snapshotsPaneRef.current;
      if (!root || !id) return;
      const btn = root.querySelector<HTMLButtonElement>(
        `button[data-snapshot-id="${CSS.escape(id)}"]`,
      );
      btn?.focus();
    });
  }

  return (
    <main className={`app${selectedSnapshot ? " has-files" : ""}`}>
      <h1>restarilo</h1>

      <label className="app-repo">
        Repo{" "}
        <select
          value={selectedRepo}
          onChange={(e) => setSelectedRepo(e.target.value)}
          disabled={repos.length === 0 || actionBusy}
        >
          {repos.map((r) => (
            <option key={r.name} value={r.name}>
              {r.name}
            </option>
          ))}
        </select>
      </label>

      {actionBusy && (
        <div className="app-action-bar">
          <span className="file-actions-busy">Working…</span>
          <button type="button" onClick={onCancel}>
            Cancel
          </button>
        </div>
      )}

      <div className="app-panes">
        <section className="app-pane app-pane-snapshots" ref={snapshotsPaneRef}>
          <h2>Snapshots</h2>
          <div className="app-pane-body">
            <SnapshotList
              snapshots={snapshots}
              selectedId={selectedSnapshot?.id ?? null}
              loading={loading}
              error={error}
              onSelectSnapshot={selectSnapshot}
            />
          </div>
        </section>
        <section className="app-pane app-pane-files" ref={filesPaneRef}>
          <div className="app-pane-files-head">
            <h2>Files</h2>
            {selectedSnapshot && (
              <button
                type="button"
                className="linkish app-back-snapshots"
                onClick={backToSnapshots}
              >
                Snapshots
              </button>
            )}
          </div>
          <div className="app-pane-body app-pane-body-files">
            {selectedSnapshot ? (
              <FileBrowser
                key={`${selectedRepo}:${selectedSnapshot.id}`}
                repoName={selectedRepo}
                snapshotId={selectedSnapshot.id}
                actionBusy={actionBusy}
                onActionId={setActionId}
              />
            ) : (
              <p className="app-loading">Select a snapshot to browse files.</p>
            )}
          </div>
        </section>
      </div>
    </main>
  );
}

export default App;
