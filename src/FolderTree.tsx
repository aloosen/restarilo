import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import {
  ancestorPaths,
  applyFolderDirsResult,
  flattenVisible,
  mergeBranchFromEvent,
  type BranchState,
} from "./folderTreeNav";
import { FolderTreeNode } from "./FolderTreeNode";
import {
  loadFolderDirs,
  mergeFolderDirs,
  subscribeFolderIndex,
} from "./folderIndex";

type Props = {
  repoName: string;
  snapshotId: string;
  currentPath: string;
  onNavigate: (path: string) => void;
};

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
  const treeRef = useRef<HTMLUListElement>(null);
  identityRef.current = identity;

  useEffect(() => {
    setExpanded(new Set(["/"]));
    setBranches({});
    branchGen.current = {};
  }, [repoName, snapshotId]);

  useEffect(() => {
    let alive = true;
    let unlisten: (() => void) | undefined;
    void subscribeFolderIndex(repoName, snapshotId, (ev) => {
      if (!alive || identityRef.current !== `${repoName}\0${snapshotId}`) return;
      setBranches((prev) => {
        const cur = prev[ev.parentPath];
        const dirs = mergeFolderDirs(cur?.dirs ?? [], ev.dirs);
        return {
          ...prev,
          [ev.parentPath]: mergeBranchFromEvent(cur, dirs, ev.childrenComplete),
        };
      });
    }).then((u) => {
      unlisten = u;
    });
    return () => {
      alive = false;
      unlisten?.();
    };
  }, [repoName, snapshotId]);

  const loadBranch = useCallback(
    (branchPath: string, prioritize: boolean) => {
      const gen = (branchGen.current[branchPath] ?? 0) + 1;
      branchGen.current[branchPath] = gen;
      const startedIdentity = identityRef.current;
      setBranches((prev) => {
        const cur = prev[branchPath];
        const dirs = cur?.dirs ?? [];
        // Keep existing children visible; only spin when this branch is still empty.
        return {
          ...prev,
          [branchPath]: {
            loading: dirs.length === 0,
            error: null,
            dirs,
            childrenComplete: cur?.childrenComplete ?? false,
          },
        };
      });
      loadFolderDirs(repoName, snapshotId, branchPath, prioritize)
        .then((res) => {
          if (identityRef.current !== startedIdentity) return;
          if (branchGen.current[branchPath] !== gen) return;
          setBranches((prev) => ({
            ...prev,
            [branchPath]: applyFolderDirsResult(
              mergeFolderDirs(prev[branchPath]?.dirs ?? [], res.dirs),
              res.children_complete,
              null,
            ),
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
              childrenComplete: prev[branchPath]?.childrenComplete ?? false,
            },
          }));
        });
    },
    [repoName, snapshotId],
  );

  useEffect(() => {
    loadBranch("/", false);
  }, [loadBranch, repoName, snapshotId]);

  useEffect(() => {
    const parts = ancestorPaths(currentPath);
    setExpanded((prev) => {
      const next = new Set(prev);
      for (const p of parts) next.add(p);
      return next;
    });
    for (const p of parts) loadBranch(p, false);
  }, [currentPath, loadBranch]);

  function toggle(path: string) {
    setExpanded((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else {
        next.add(path);
        loadBranch(path, true);
      }
      return next;
    });
  }

  function navigate(path: string) {
    loadBranch(path, true);
    onNavigate(path);
  }

  function focusPath(path: string) {
    treeRef.current
      ?.querySelector<HTMLButtonElement>(
        `button.folder-tree-name[data-path="${CSS.escape(path)}"]`,
      )
      ?.focus();
  }

  function onKey(e: KeyboardEvent, path: string) {
    if (e.key === "Enter") {
      e.preventDefault();
      navigate(path);
      return;
    }
    if (e.key === "ArrowRight") {
      e.preventDefault();
      if (!expanded.has(path)) toggle(path);
      return;
    }
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      if (expanded.has(path)) toggle(path);
      return;
    }
    const visible = flattenVisible("/", expanded, branches);
    const i = visible.indexOf(path);
    if (e.key === "Home") {
      e.preventDefault();
      focusPath(visible[0] ?? "/");
      return;
    }
    if (e.key === "End") {
      e.preventDefault();
      focusPath(visible[visible.length - 1] ?? "/");
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = e.key === "ArrowDown" ? i + 1 : i - 1;
      if (next < 0 || next >= visible.length) return;
      focusPath(visible[next]);
    }
  }

  return (
    <ul className="folder-tree" role="tree" ref={treeRef}>
      <FolderTreeNode
        path="/"
        name="/"
        depth={0}
        currentPath={currentPath}
        expanded={expanded}
        branches={branches}
        onToggle={toggle}
        onNavigate={navigate}
        onRetry={(p) => loadBranch(p, true)}
        onKey={onKey}
      />
    </ul>
  );
}
