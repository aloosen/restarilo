export function explorerEmptyMessage(o: {
  configError: string | null;
  zeroRepos: boolean;
  shareStatus: boolean;
  snapError: boolean;
  emptyRepo: boolean;
}): string {
  if (o.configError || o.zeroRepos)
    return "Configure a repository to browse snapshots.";
  if (o.shareStatus) return "Network share access required.";
  if (o.snapError)
    return "Could not load snapshots. Use Retry next to the snapshot control.";
  if (o.emptyRepo) return "No snapshots in this repository.";
  return "Loading snapshots…";
}
