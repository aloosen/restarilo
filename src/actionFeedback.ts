export type ActionKind = "restore" | "export" | "open" | "search" | "tags";

export function busyLabelFor(kind: ActionKind): string {
  if (kind === "restore") return "Restoring…";
  if (kind === "export") return "Exporting…";
  if (kind === "search") return "Searching…";
  if (kind === "tags") return "Saving tags…";
  return "Opening…";
}

export function successMessage(kind: ActionKind, dest: string): string {
  if (kind === "restore") return `Restored to ${dest}`;
  if (kind === "export") return `Exported to ${dest}`;
  if (kind === "search" || kind === "tags") return dest;
  return `Opened ${dest}`;
}

export function targetFromExistsError(message: string): string | null {
  const prefix = "TARGET_EXISTS:";
  if (!message.startsWith(prefix)) return null;
  return message.slice(prefix.length);
}

export function joinTarget(targetDir: string, name: string, zip: boolean): string {
  const sep = targetDir.includes("\\") && !targetDir.includes("/") ? "\\" : "/";
  const base = targetDir.endsWith("/") || targetDir.endsWith("\\")
    ? targetDir.slice(0, -1)
    : targetDir;
  return zip ? `${base}${sep}${name}.zip` : `${base}${sep}${name}`;
}
