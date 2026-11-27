import type { SearchHit } from "./SearchOverlay";
import {
  runRestoreOrDump,
  type PathActionHooks,
} from "./pathActionRun";

export async function exportSearchHit(opts: {
  repoName: string;
  hit: SearchHit;
  snapshotLabel: string;
  hooks: PathActionHooks;
  askOverwrite: (args: {
    sourcePath: string;
    snapshotLabel: string;
    targetPath: string;
  }) => Promise<boolean>;
}): Promise<void> {
  const selection = {
    path: opts.hit.path,
    name: opts.hit.name,
    is_dir: opts.hit.is_dir,
  };
  await runRestoreOrDump({
    cmd: "dump_path",
    frozen: {
      repoName: opts.repoName,
      snapshotId: opts.hit.snapshot_id,
      snapshotLabel: opts.snapshotLabel,
      selection,
    },
    live: {
      repoName: opts.repoName,
      snapshotId: opts.hit.snapshot_id,
      selection,
    },
    hooks: opts.hooks,
    askOverwrite: opts.askOverwrite,
  });
}
