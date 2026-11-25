import { useEffect, useRef, type KeyboardEvent } from "react";
import { formatMtime, formatSize } from "./formatSize";
import type { DirEntry } from "./listDirCache";
import { FileIcon, FolderIcon } from "./listingIcons";

type Props = {
  path: string;
  entries: DirEntry[];
  selectedPath: string | null;
  onSelect: (entry: DirEntry) => void;
  onNavigate: (path: string) => void;
};

function sortEntries(rows: DirEntry[]): DirEntry[] {
  return [...rows].sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    if (a.name < b.name) return -1;
    if (a.name > b.name) return 1;
    return 0;
  });
}

export function FileListing({
  path,
  entries,
  selectedPath,
  onSelect,
  onNavigate,
}: Props) {
  const listRef = useRef<HTMLDivElement>(null);
  const sorted = sortEntries(entries);

  useEffect(() => {
    const first = listRef.current?.querySelector<HTMLButtonElement>(
      "button.file-listing-row",
    );
    first?.focus();
  }, [path, entries]);

  function rowButtons(): HTMLButtonElement[] {
    const root = listRef.current;
    if (!root) return [];
    return Array.from(
      root.querySelectorAll<HTMLButtonElement>("button.file-listing-row"),
    );
  }

  function onRowKey(e: KeyboardEvent, entry: DirEntry, index: number) {
    if (e.key === " ") {
      e.preventDefault();
      onSelect(entry);
      return;
    }
    if (e.key === "Enter" && entry.is_dir) {
      e.preventDefault();
      onNavigate(entry.path);
      return;
    }
    if (e.key === "Home" || e.key === "End") {
      e.preventDefault();
      const i = e.key === "Home" ? 0 : sorted.length - 1;
      if (i < 0) return;
      rowButtons()[i]?.focus();
      onSelect(sorted[i]);
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = e.key === "ArrowDown" ? index + 1 : index - 1;
      if (next < 0 || next >= sorted.length) return;
      rowButtons()[next]?.focus();
      onSelect(sorted[next]);
    }
  }

  return (
    <div className="file-listing" ref={listRef}>
      <table className="app-table">
        <thead>
          <tr>
            <th>Name</th>
            <th className="col-size">Size</th>
            <th className="col-mtime">Modified</th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((e, index) => {
            const selected = selectedPath === e.path;
            return (
              <tr
                key={e.path}
                className={
                  selected ? "is-selectable is-selected" : "is-selectable"
                }
                onClick={() => onSelect(e)}
                onDoubleClick={() => e.is_dir && onNavigate(e.path)}
              >
                <td>
                  <button
                    type="button"
                    className="linkish file-listing-row"
                    aria-current={selected ? "true" : undefined}
                    onClick={(event) => {
                      event.stopPropagation();
                      onSelect(e);
                    }}
                    onDoubleClick={(event) => {
                      event.stopPropagation();
                      if (e.is_dir) onNavigate(e.path);
                    }}
                    onKeyDown={(ev) => onRowKey(ev, e, index)}
                  >
                    {e.is_dir ? (
                      <FolderIcon className="listing-icon listing-icon-folder" />
                    ) : (
                      <FileIcon className="listing-icon" />
                    )}
                    <span className="listing-name">{e.name}</span>
                  </button>
                </td>
                <td className="col-size mono">
                  {e.is_dir ? "–" : formatSize(e.size) || "–"}
                </td>
                <td className="col-mtime mono">{formatMtime(e.mtime) || "–"}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

export function breadcrumbParts(
  path: string,
): { label: string; target: string }[] {
  const parts = [{ label: "All files", target: "/" }];
  if (path === "/" || path === "") return parts;
  const segs = path.split("/").filter(Boolean);
  let acc = "";
  for (const seg of segs) {
    acc += `/${seg}`;
    parts.push({ label: seg, target: acc });
  }
  return parts;
}

export function folderHeading(path: string): string {
  if (path === "/" || path === "") return "All files";
  const segs = path.split("/").filter(Boolean);
  return segs[segs.length - 1] ?? path;
}
