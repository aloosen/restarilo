import type { DirEntry } from "./listDirCache";

export type BranchState = {
  loading: boolean;
  error: string | null;
  dirs: DirEntry[];
};

export function flattenVisible(
  path: string,
  expanded: Set<string>,
  branches: Record<string, BranchState>,
): string[] {
  const out = [path];
  if (!expanded.has(path)) return out;
  const branch = branches[path];
  if (!branch || branch.loading || branch.error) return out;
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
