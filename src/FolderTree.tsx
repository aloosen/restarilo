import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import { loadListDir, type DirEntry } from "./listDirCache";

type Props = {
  repoName: string;
  snapshotId: string;
  currentPath: string;
  onNavigate: (path: string) => void;
};

type BranchState = {
  loading: boolean;
  error: string | null;
  dirs: DirEntry[];
};

function ancestorPaths(path: string): string[] {
  if (path === "/" || path === "") return ["/"];
  const out = ["/"];
  let acc = "";
  for (const seg of path.split("/").filter(Boolean)) {
    acc += `/${seg}`;
    out.push(acc);
  }
  return out;
}

export function FolderTree({
  repoName,
  snapshotId,
  currentPath,
  onNavigate,
}: Props) {
  const identity = `${repoName}\0${snapshotId}`;
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set(["/"]));
  const [branches, setBranches] = useState<Record<string, BranchState>>({});
  const branchGen = useRef<Record<string, number>>({});
  const identityRef = useRef(identity);
  identityRef.current = identity;

  useEffect(() => {
    setExpanded(new Set(["/"]));
    setBranches({});
    branchGen.current = {};
  }, [repoName, snapshotId]);

  const loadBranch = useCallback((branchPath: string) => {
    const gen = (branchGen.current[branchPath] ?? 0) + 1;
    branchGen.current[branchPath] = gen;
    const startedIdentity = identityRef.current;
    const repo = repoName;
    const snap = snapshotId;
    setBranches((prev) => ({
      ...prev,
      [branchPath]: {
        loading: true,
        error: null,
        dirs: prev[branchPath]?.dirs ?? [],
      },
    }));
    loadListDir(repo, snap, branchPath)
      .then((rows) => {
        if (identityRef.current !== startedIdentity) return;
        if (branchGen.current[branchPath] !== gen) return;
        const dirs = rows
          .filter((e) => e.is_dir)
          .sort((a, b) => a.name.localeCompare(b.name));
        setBranches((prev) => ({
          ...prev,
          [branchPath]: { loading: false, error: null, dirs },
        }));
      })
      .catch((e: unknown) => {
        if (identityRef.current !== startedIdentity) return;
        if (branchGen.current[branchPath] !== gen) return;
        setBranches((prev) => ({
          ...prev,
          [branchPath]: {
            loading: false,
            error: String(e),
            dirs: prev[branchPath]?.dirs ?? [],
          },
        }));
      });
  }, [repoName, snapshotId]);

  useEffect(() => {
    loadBranch("/");
  }, [loadBranch, repoName, snapshotId]);

  useEffect(() => {
    const parts = ancestorPaths(currentPath);
    setExpanded((prev) => {
      const next = new Set(prev);
      for (const p of parts) next.add(p);
      return next;
    });
    for (const p of parts) loadBranch(p);
  }, [currentPath, loadBranch]);

  function toggle(path: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else {
        next.add(path);
        loadBranch(path);
      }
      return next;
    });
  }

  function onKey(e: KeyboardEvent, path: string) {
    if (e.key === "Enter") {
      e.preventDefault();
      onNavigate(path);
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      if (!expanded.has(path)) toggle(path);
    } else if (e.key === "ArrowLeft") {
      e.preventDefault();
      if (expanded.has(path)) toggle(path);
    }
  }

  return (
    <ul className="folder-tree" role="tree">
      <TreeNode
        path="/"
        name="/"
        depth={0}
        currentPath={currentPath}
        expanded={expanded}
        branches={branches}
        onToggle={toggle}
        onNavigate={onNavigate}
        onRetry={loadBranch}
        onKey={onKey}
      />
    </ul>
  );
}

type NodeProps = {
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

function TreeNode(p: NodeProps) {
  const open = p.expanded.has(p.path);
  const branch = p.branches[p.path];
  const active = p.currentPath === p.path;
  return (
    <li role="treeitem" aria-expanded={open} className={active ? "is-active" : undefined}>
      <div className="folder-tree-row" style={{ paddingLeft: p.depth * 12 }}>
        <button
          type="button"
          className="folder-tree-twist"
          aria-label={open ? "Collapse" : "Expand"}
          onClick={() => p.onToggle(p.path)}
        >
          {open ? "▾" : "▸"}
        </button>
        <button
          type="button"
          className="linkish"
          onClick={() => p.onNavigate(p.path)}
          onKeyDown={(e) => p.onKey(e, p.path)}
        >
          {p.name}
        </button>
      </div>
      {open && branch?.loading && (
        <p className="app-loading" style={{ paddingLeft: (p.depth + 1) * 12 }}>Loading…</p>
      )}
      {open && branch?.error && (
        <div style={{ paddingLeft: (p.depth + 1) * 12 }}>
          <pre className="app-error">{branch.error}</pre>
          <button type="button" onClick={() => p.onRetry(p.path)}>Retry</button>
        </div>
      )}
      {open && branch && !branch.loading && !branch.error && (
        <ul role="group">
          {branch.dirs.map((d) => (
            <TreeNode key={d.path} {...p} path={d.path} name={d.name} depth={p.depth + 1} />
          ))}
        </ul>
      )}
    </li>
  );
}
