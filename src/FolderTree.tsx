import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import {
  ancestorPaths,
  applyFolderDirsResult,
  flattenVisible,
  type BranchState,
} from "./folderTreeNav";
import { FolderTreeNode } from "./FolderTreeNode";
import { loadFolderDirs, mergeFolderDirs } from "./folderIndex";
import { useFolderIndexSubscribe } from "./useFolderIndexSubscribe";

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
  const activeGeneration = useRef<number | null>(null);
  const identityRef = useRef(identity);
  const treeRef = useRef<HTMLUListElement>(null);
  identityRef.current = identity;

  useEffect(() => {
    setExpanded(new Set(["/"]));
    setBranches({});
    branchGen.current = {};
    activeGeneration.current = null;
  }, [repoName, snapshotId]);

  useFolderIndexSubscribe(
    repoName,
    snapshotId,
    activeGeneration,
    identityRef,
    setBranches,
  );

  const loadBranch = useCallback(
    (branchPath: string, prioritize: boolean) => {
      const gen = (branchGen.current[branchPath] ?? 0) + 1;
      branchGen.current[branchPath] = gen;
      const startedIdentity = identityRef.current;
      setBranches((prev) => {
        const cur = prev[branchPath];
        const dirs = cur?.dirs ?? [];
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
          activeGeneration.current = res.generation;
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
    const collapsing = expanded.has(path);
    setExpanded((prev) => {
      const next = new Set(prev);
      if (collapsing) next.delete(path);
      else next.add(path);
      return next;
    });
    if (!collapsing) loadBranch(path, true);
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
