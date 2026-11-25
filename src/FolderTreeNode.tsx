import type { KeyboardEvent } from "react";
import type { BranchState } from "./folderTreeNav";
import { icons, TablerSvg } from "./tablerIcons";

type Props = {
  path: string;
  name: string;
  depth: number;
  currentPath: string;
  expanded: Set<string>;
  branches: Record<string, BranchState>;
  onToggle: (p: string) => void;
  onNavigate: (p: string) => void;
  onRetry: (p: string) => void;
  onKey: (e: KeyboardEvent, p: string) => void;
};

export function FolderTreeNode(p: Props) {
  const open = p.expanded.has(p.path);
  const branch = p.branches[p.path];
  const isRoot = p.path === "/";
  const label = isRoot ? "All files" : p.name;

  return (
    <li
      role="treeitem"
      aria-expanded={open}
      className={p.currentPath === p.path ? "is-active" : undefined}
    >
      <div className="folder-tree-row" style={{ paddingLeft: p.depth * 12 }}>
        <button
          type="button"
          className="folder-tree-twist"
          aria-label={open ? "Collapse" : "Expand"}
          onClick={() => p.onToggle(p.path)}
        >
          <TablerSvg
            svg={open ? icons.chevronDown : icons.chevronRight}
            className="folder-tree-twist-icon"
          />
        </button>
        <button
          type="button"
          className="linkish folder-tree-name"
          data-path={p.path}
          onClick={() => p.onNavigate(p.path)}
          onKeyDown={(e) => p.onKey(e, p.path)}
        >
          <TablerSvg
            svg={isRoot ? icons.drive : icons.folder}
            className={
              isRoot ? "folder-tree-icon" : "folder-tree-icon listing-icon-folder"
            }
          />
          <span>{label}</span>
        </button>
      </div>
      {open && branch?.loading && (
        <p className="app-loading" style={{ paddingLeft: (p.depth + 1) * 12 }}>
          Loading…
        </p>
      )}
      {open && branch?.error && (
        <div style={{ paddingLeft: (p.depth + 1) * 12 }}>
          <pre className="app-error">{branch.error}</pre>
          <button type="button" onClick={() => p.onRetry(p.path)}>
            Retry
          </button>
        </div>
      )}
      {open && branch && !branch.loading && !branch.error && (
        <ul role="group">
          {branch.dirs.map((d) => (
            <FolderTreeNode
              key={d.path}
              {...p}
              path={d.path}
              name={d.name}
              depth={p.depth + 1}
            />
          ))}
        </ul>
      )}
    </li>
  );
}
