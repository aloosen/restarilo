import type { DirEntry } from "./listDirCache";

export type BranchState = {
  loading: boolean;
  error: string | null;
  dirs: DirEntry[];
  childrenComplete: boolean;
};

export function flattenVisible(
  path: string,
  expanded: Set<string>,
  branches: Record<string, BranchState>,
): string[] {
  const out = [path];
  if (!expanded.has(path)) return out;
  const branch = branches[path];
  if (!branch) return out;
  for (const d of branch.dirs) {
    out.push(...flattenVisible(d.path, expanded, branches));
  }
  return out;
}

export function ancestorPaths(path: string): string[] {
  if (path === "/" || path === "") return ["/"];
  const out = ["/"];
  let acc = "";
  for (const seg of path.split("/").filter(Boolean)) {
    acc += `/${seg}`;
    out.push(acc);
  }
  return out;
}

export function applyFolderDirsResult(
  dirs: DirEntry[],
  childrenComplete: boolean,
  error: string | null,
): BranchState {
  return {
    loading: error ? false : !childrenComplete,
    error,
    dirs,
    childrenComplete,
  };
}

/** One-level listing complete must stick; progressive scan emits never clear it. */
export function mergeBranchFromEvent(
  prev: BranchState | undefined,
  dirs: DirEntry[],
  eventChildrenComplete: boolean,
): BranchState {
  const childrenComplete =
    prev?.childrenComplete === true || eventChildrenComplete;
  return applyFolderDirsResult(dirs, childrenComplete, null);
}
