import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type RepoName = { name: string };

type SnapshotRow = {
  id: string;
  time: string;
  hostname: string;
  paths: string[];
  tags: string[];
};

function App() {
  const [repos, setRepos] = useState<RepoName[]>([]);
  const [selected, setSelected] = useState("");
  const [snapshots, setSnapshots] = useState<SnapshotRow[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    invoke<RepoName[]>("list_repos")
      .then((list) => {
        setRepos(list);
        if (list[0]) setSelected(list[0].name);
      })
      .catch((e: unknown) => setError(String(e)));
  }, []);

  useEffect(() => {
    if (!selected) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    invoke<SnapshotRow[]>("list_snapshots", { repoName: selected })
      .then((rows) => {
        if (!cancelled) setSnapshots(rows);
      })
      .catch((e: unknown) => {
        if (!cancelled) {
          setSnapshots([]);
          setError(String(e));
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [selected]);

  return (
    <main style={{ fontFamily: "system-ui", padding: 16 }}>
      <h1 style={{ margin: "0 0 12px", fontSize: 18 }}>restarilo</h1>

      <label style={{ display: "block", marginBottom: 12 }}>
        Repo{" "}
        <select
          value={selected}
          onChange={(e) => setSelected(e.target.value)}
          disabled={repos.length === 0}
        >
          {repos.map((r) => (
            <option key={r.name} value={r.name}>
              {r.name}
            </option>
          ))}
        </select>
      </label>

      {error && <pre style={{ color: "crimson" }}>{error}</pre>}
      {loading && <p>Loading snapshots…</p>}

      {!loading && !error && (
        <table style={{ borderCollapse: "collapse", width: "100%" }}>
          <thead>
            <tr>
              <th align="left">ID</th>
              <th align="left">Time</th>
              <th align="left">Host</th>
              <th align="left">Paths</th>
            </tr>
          </thead>
          <tbody>
            {snapshots.map((s) => (
              <tr key={s.id + s.time}>
                <td style={{ fontFamily: "monospace" }}>{s.id}</td>
                <td>{s.time}</td>
                <td>{s.hostname}</td>
                <td>{s.paths.join(", ")}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </main>
  );
}

export default App;
