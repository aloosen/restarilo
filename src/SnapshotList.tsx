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
        {snapshots.map((s) => {
          const label = s.short_id || s.id.slice(0, 8);
          const selected = selectedId === s.id;
          return (
            <tr key={s.id} className={selected ? "is-selected" : undefined}>
              <td className="mono">
                <button
                  type="button"
                  className="linkish"
                  data-snapshot-id={s.id}
                  aria-current={selected ? "true" : undefined}
                  onClick={() => onSelectSnapshot(s)}
                >
                  {label}
                </button>
              </td>
              <td>{s.time}</td>
              <td>{s.hostname}</td>
              <td>{s.paths.join(", ")}</td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
