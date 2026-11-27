/** Format byte counts for the listing (KiB/MiB/…, 1 decimal when ≥ 1 KiB). */
export function formatSize(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined) return "";
  if (!Number.isFinite(bytes) || bytes < 0) return "";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KiB", "MiB", "GiB", "TiB"] as const;
  let v = bytes / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i += 1;
  }
  const rounded = v >= 10 || i === 0 ? v.toFixed(0) : v.toFixed(1);
  return `${rounded} ${units[i]}`;
}

function parsedDate(raw: string): Date | null {
  const ms = Date.parse(raw);
  if (!Number.isFinite(ms)) return null;
  return new Date(ms);
}

function pad(n: number): string {
  return String(n).padStart(2, "0");
}

/** Full local timestamp (for title / detail). */
export function formatMtimeFull(raw: string | null | undefined): string {
  if (!raw) return "";
  const d = parsedDate(raw);
  if (!d) return raw;
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

/** Short one-line listing date (no seconds). */
export function formatMtime(raw: string | null | undefined): string {
  if (!raw) return "";
  const d = parsedDate(raw);
  if (!d) return raw;
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** Snapshot select primary text: date+time without seconds. */
export function formatSnapshotWhen(time: string): string {
  return formatMtime(time) || time;
}

/** Readable snapshot label for overwrite / chrome (when + host + short id). */
export function formatSnapshotLabel(s: {
  id: string;
  short_id: string;
  time: string;
  hostname: string;
}): string {
  const when = formatSnapshotWhen(s.time);
  const short = s.short_id || s.id.slice(0, 8);
  const host = s.hostname?.trim();
  return host ? `${when} · ${host} · ${short}` : `${when} · ${short}`;
}

/** Closed select option: date+time + hostname; short/full id in title. */
export function formatSnapshotOption(s: {
  id: string;
  short_id: string;
  time: string;
  hostname: string;
}): { text: string; title: string } {
  const when = formatSnapshotWhen(s.time);
  const short = s.short_id || s.id.slice(0, 8);
  const host = s.hostname?.trim();
  const text = host ? `${when} · ${host}` : when;
  const title = host
    ? `${when} · ${host} · ${short} (${s.id})`
    : `${when} · ${short} (${s.id})`;
  return { text, title };
}

