export type SnapshotRow = {
  id: string;
  short_id: string;
  time: string;
  hostname: string;
  paths: string[];
  tags: string[];
};

type Props = {
  snapshots: SnapshotRow[];
  selectedId: string | null;
  loading: boolean;
  error: string | null;
  onSelectSnapshot: (row: SnapshotRow) => void;
};

export function SnapshotList({
  snapshots,
  selectedId,
  loading,
  error,
  onSelectSnapshot,
}: Props) {
  if (error) return <pre className="app-error">{error}</pre>;
  if (loading) return <p className="app-loading">Loading snapshots…</p>;

  return (
    <table className="app-table">
      <thead>
        <tr>
          <th>ID</th>
          <th>Time</th>
          <th>Host</th>
          <th>Paths</th>
        </tr>
      </thead>
      <tbody>
        {snapshots.map((s) => (
          <tr
            key={s.id}
            className={selectedId === s.id ? "is-selected" : undefined}
            onClick={() => onSelectSnapshot(s)}
            style={{ cursor: "pointer" }}
          >
            <td className="mono">{s.short_id || s.id.slice(0, 8)}</td>
            <td>{s.time}</td>
            <td>{s.hostname}</td>
            <td>{s.paths.join(", ")}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
