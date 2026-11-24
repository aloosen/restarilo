import { useEffect, useState } from "react";
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

  return (
    <main className="app">
      <h1>restarilo</h1>

      <label className="app-repo">
        Repo{" "}
        <select
          value={selectedRepo}
          onChange={(e) => setSelectedRepo(e.target.value)}
          disabled={repos.length === 0}
        >
          {repos.map((r) => (
            <option key={r.name} value={r.name}>
              {r.name}
            </option>
          ))}
        </select>
      </label>

      <div className="app-panes">
        <section className="app-pane">
          <h2>Snapshots</h2>
          <SnapshotList
            snapshots={snapshots}
            selectedId={selectedSnapshot?.id ?? null}
            loading={loading}
            error={error}
            onSelectSnapshot={setSelectedSnapshot}
          />
        </section>
        <section className="app-pane">
          <h2>Files</h2>
          {selectedSnapshot ? (
            <FileBrowser
              key={`${selectedRepo}:${selectedSnapshot.id}`}
              repoName={selectedRepo}
              snapshotId={selectedSnapshot.id}
            />
          ) : (
            <p className="app-loading">Select a snapshot to browse files.</p>
          )}
        </section>
      </div>
    </main>
  );
}

export default App;
