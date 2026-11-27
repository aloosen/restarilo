import { formatSnapshotOption } from "./formatSize";

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
  const orphanId = selectedId && !selected ? selectedId : null;
  const selectTitle = selected
    ? formatSnapshotOption(selected).title
    : orphanId
      ? `${orphanId} (pending)`
      : undefined;

  return (
    <div className="app-snapshot">
      <label>
        Snapshot{" "}
        <select
          value={selectedId ?? ""}
          disabled={disabled || loading || empty || !!listError}
          title={selectTitle}
          onChange={(e) => onSelectId(e.target.value)}
        >
          {(empty || listError) && (
            <option value="">{listError ? "Unavailable" : "No snapshots"}</option>
          )}
          {orphanId && (
            <option value={orphanId}>
              {orphanId.slice(0, 8)}… (pending)
            </option>
          )}
          {!selected && !orphanId && !empty && !listError && (
            <option value="">Select snapshot…</option>
          )}
          {sorted.map((s) => {
            const opt = formatSnapshotOption(s);
            return (
              <option key={s.id} value={s.id} title={opt.title}>
                {opt.text}
              </option>
            );
          })}
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
