import type { ShareAccessStatus } from "./shareAccess";
import { ShareAccessPanel } from "./ShareAccessPanel";
import { SnapshotList, type SnapshotRow } from "./SnapshotList";

type RepoName = { name: string };

type Props = {
  configError: string | null;
  zeroRepos: boolean;
  repos: RepoName[];
  selectedRepo: string;
  onSelectRepo: (name: string) => void;
  chromeLocked: boolean;
  snapshots: SnapshotRow[];
  selectedSnapshotId: string | null;
  loading: boolean;
  snapError: string | null;
  onSelectSnapshotId: (id: string) => void;
  onRetryList: () => void;
  shareStatus: ShareAccessStatus | null;
  onSignIn: () => void;
  onTryAnotherAccount: () => void;
};

export function ExplorerChromeControls(p: Props) {
  if (p.configError) {
    return (
      <p className="app-error app-config-error">
        Could not load configuration. Fix config.toml and restart restarilo.
        {"\n"}
        {p.configError}
      </p>
    );
  }
  if (p.zeroRepos) {
    return (
      <p className="app-loading">
        No repositories configured. Add [[repos]] to config.toml and restart
        restarilo.
      </p>
    );
  }
  return (
    <>
      <label className="app-repo">
        Repo{" "}
        <select
          value={p.selectedRepo}
          onChange={(e) => p.onSelectRepo(e.target.value)}
          disabled={p.repos.length === 0 || p.chromeLocked}
        >
          {p.repos.map((r) => (
            <option key={r.name} value={r.name}>
              {r.name}
            </option>
          ))}
        </select>
      </label>
      <SnapshotList
        snapshots={p.snapshots}
        selectedId={p.selectedSnapshotId}
        loading={p.loading}
        disabled={p.chromeLocked}
        listError={p.snapError}
        onSelectId={p.onSelectSnapshotId}
        onRetryList={p.onRetryList}
      />
      <ShareAccessPanel
        status={p.shareStatus}
        disabled={p.chromeLocked || p.loading}
        onSignIn={p.onSignIn}
        onTryAnotherAccount={p.onTryAnotherAccount}
        onRetry={p.onRetryList}
      />
    </>
  );
}
