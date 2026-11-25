import { formatSnapshotLabel } from "./formatSize";

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
  /** Failed list_snapshots — distinct from empty repo. */
  listError: string | null;
  onSelectId: (id: string) => void;
  onRetryList?: () => void;
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

export function SnapshotList({
  snapshots,
  selectedId,
  loading,
  disabled,
  listError,
  onSelectId,
  onRetryList,
}: Props) {
  const sorted = sortSnapshotsNewestFirst(snapshots);
  const empty = sorted.length === 0 && !listError;
  const selected = sorted.find((s) => s.id === selectedId);

  return (
    <div className="app-snapshot">
      <label>
        Snapshot{" "}
        <select
          value={selectedId ?? ""}
          disabled={disabled || loading || empty || !!listError}
          title={selected ? selected.id : undefined}
          onChange={(e) => onSelectId(e.target.value)}
        >
          {(empty || listError) && (
            <option value="">{listError ? "Unavailable" : "No snapshots"}</option>
          )}
          {sorted.map((s) => (
            <option key={s.id} value={s.id} title={s.id}>
              {formatSnapshotLabel(s)}
            </option>
          ))}
        </select>
      </label>
      {loading && <span className="app-loading">Loading snapshots…</span>}
      {listError && (
        <div className="app-snap-error">
          <pre className="app-error">{listError}</pre>
          {onRetryList && (
            <button type="button" onClick={onRetryList} disabled={disabled || loading}>
              Retry
            </button>
          )}
        </div>
      )}
    </div>
  );
}
