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
  disabled?: boolean;
  error: string | null;
  onSelectId: (id: string) => void;
};

function instantMs(time: string): number {
  const n = Date.parse(time);
  return Number.isFinite(n) ? n : 0;
}

export function sortSnapshotsNewestFirst(rows: SnapshotRow[]): SnapshotRow[] {
  return [...rows].sort((a, b) => {
    const diff = instantMs(b.time) - instantMs(a.time);
    if (diff !== 0) return diff;
    if (a.id < b.id) return -1;
    if (a.id > b.id) return 1;
    return 0;
  });
}

function optionLabel(s: SnapshotRow): string {
  const short = s.short_id || s.id.slice(0, 8);
  return `${s.time} · ${s.hostname} · ${short}`;
}

export function SnapshotList({
  snapshots,
  selectedId,
  loading,
  disabled,
  error,
  onSelectId,
}: Props) {
  const sorted = sortSnapshotsNewestFirst(snapshots);
  const empty = sorted.length === 0;

  return (
    <div className="app-snapshot">
      <label>
        Snapshot{" "}
        <select
          value={selectedId ?? ""}
          disabled={disabled || loading || empty}
          onChange={(e) => onSelectId(e.target.value)}
        >
          {empty && <option value="">No snapshots</option>}
          {sorted.map((s) => (
            <option key={s.id} value={s.id}>
              {optionLabel(s)}
            </option>
          ))}
        </select>
      </label>
      {loading && <span className="app-loading">Loading snapshots…</span>}
      {error && <pre className="app-error">{error}</pre>}
    </div>
  );
}
