import { useEffect, useRef, type KeyboardEvent } from "react";
import type { DirEntry } from "./listDirCache";

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

function breadcrumbParts(path: string): { label: string; target: string }[] {
  const parts = [{ label: "/", target: "/" }];
  if (path === "/" || path === "") return parts;
  const segs = path.split("/").filter(Boolean);
  let acc = "";
  for (const seg of segs) {
    acc += `/${seg}`;
    parts.push({ label: seg, target: acc });
  }
  return parts;
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
  const crumbs = breadcrumbParts(path);

  useEffect(() => {
    const first = listRef.current?.querySelector<HTMLButtonElement>("button.linkish");
    first?.focus();
  }, [path, entries]);

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
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = e.key === "ArrowDown" ? index + 1 : index - 1;
      if (next < 0 || next >= sorted.length) return;
      const btn = listRef.current?.querySelectorAll<HTMLButtonElement>("button.linkish")[
        next
      ];
      btn?.focus();
      onSelect(sorted[next]);
    }
  }

  return (
    <div className="file-listing" ref={listRef}>
      <nav className="file-breadcrumb" aria-label="Path">
        {crumbs.map((c, i) => (
          <span key={c.target}>
            {i > 0 && <span className="file-breadcrumb-sep"> / </span>}
            <button
              type="button"
              className="linkish"
              disabled={c.target === path}
              onClick={() => onNavigate(c.target)}
            >
              {c.label}
            </button>
          </span>
        ))}
      </nav>
      <table className="app-table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Size</th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((e, index) => {
            const selected = selectedPath === e.path;
            const size =
              e.size === null || e.size === undefined ? "" : String(e.size);
            return (
              <tr
                key={e.path}
                className={selected ? "is-selectable is-selected" : "is-selectable"}
                onClick={() => onSelect(e)}
                onDoubleClick={() => e.is_dir && onNavigate(e.path)}
              >
                <td className="mono">
                  <button
                    type="button"
                    className="linkish"
                    aria-current={selected ? "true" : undefined}
                    onClick={(event) => {
                      if (!e.is_dir) return;
                      event.stopPropagation();
                      onNavigate(e.path);
                    }}
                    onKeyDown={(ev) => onRowKey(ev, e, index)}
                  >
                    {e.is_dir ? `${e.name}/` : e.name}
                  </button>
                </td>
                <td className="mono">{size}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
