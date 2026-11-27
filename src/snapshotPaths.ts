/** Middle-ellipsis preview: keep the start and the final path segment. */
export function formatPathPreview(path: string, maxLen = 52): string {
  const raw = path.trim();
  if (!raw) return "";
  if (raw.length <= maxLen) return raw;

  const trimmed = raw.replace(/\/+$/, "") || "/";
  const segs = trimmed.split("/").filter(Boolean);
  const last = segs.length > 0 ? segs[segs.length - 1]! : trimmed;
  const ellipsis = "…";
  const suffix = trimmed === "/" ? "/" : `/${last}`;
  const budget = maxLen - ellipsis.length - suffix.length;
  if (budget < 1) {
    if (suffix.length <= maxLen) return `${ellipsis}${suffix}`.slice(-maxLen);
    return `${ellipsis}${last.slice(-(maxLen - ellipsis.length))}`;
  }
  return `${raw.slice(0, budget)}${ellipsis}${suffix}`;
}

export function pathCountLabel(count: number): string {
  return count === 1 ? "1 path" : `${count} paths`;
}

