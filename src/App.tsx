import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import { ArchiveMark } from "./ArchiveMark";
import { BusyModal } from "./BusyModal";
import { ExplorerChromeControls } from "./ExplorerChromeControls";
import { FileBrowser } from "./FileBrowser";
import { formatSnapshotLabel, formatSnapshotTopDirs } from "./formatSize";
import { clearListDirCacheForRepo } from "./listDirCache";
import { explorerEmptyMessage } from "./explorerEmptyMessage";
import type { SearchHit } from "./SearchOverlay";
import { SnapshotTagChips } from "./SnapshotTagChips";
import { SnapshotTagEditor } from "./SnapshotTagEditor";
import { useSearchAppWire } from "./useSearchAppWire";
import { useSnapshotLoad } from "./useSnapshotLoad";
import { useSnapshotTags } from "./useSnapshotTags";
import type { ShowHitRequest } from "./usePendingShowHit";
import { useActionCancel } from "./useActionCancel";

type RepoName = { name: string };
type HeldExplorer = {
  repoName: string;
  snapshotId: string;
  snapshotLabel: string;
};

function App() {
  const [repos, setRepos] = useState<RepoName[]>([]);
  const [reposReady, setReposReady] = useState(false);
  const [configError, setConfigError] = useState<string | null>(null);
  const [selectedRepo, setSelectedRepo] = useState("");
  const [actionId, setActionId] = useState<string | null>(null);
  const [busyLabel, setBusyLabel] = useState<string | null>(null);
  const [overwriteOpen, setOverwriteOpen] = useState(false);
  const [heldExplorer, setHeldExplorer] = useState<HeldExplorer | null>(null);
  const [showHitRequest, setShowHitRequest] = useState<ShowHitRequest | null>(null);
  const [actionSuccess, setActionSuccess] = useState<string | null>(null);
  const [actionFail, setActionFail] = useState<string | null>(null);
  const actionBusy = actionId !== null;
  const cancellation = useActionCancel(actionId);
  const {
    snapshots, selectedSnapshot, snapError, shareStatus, loading,
    retryList, selectSnapshotId, refreshSelecting, prepareRepoSwitch, signIn,
    tryAnotherAccount,
  } = useSnapshotLoad(selectedRepo);

  useEffect(() => {
    invoke<RepoName[]>("list_repos")
      .then((list) => {
        setRepos(list);
        setReposReady(true);
        if (list[0]) setSelectedRepo(list[0].name);
      })
      .catch((e: unknown) => {
        setConfigError(String(e));
        setReposReady(true);
      });
  }, []);

  function onSelectRepo(name: string) {
    if (name === selectedRepo) return;
    clearListDirCacheForRepo(selectedRepo);
    prepareRepoSwitch();
    setShowHitRequest(null);
    setActionSuccess(null);
    setActionFail(null);
    setSelectedRepo(name);
  }

  const okSnap = selectedSnapshot && !snapError && !shareStatus;
  const liveRepo = okSnap ? selectedRepo : null;
  const liveSnapId = okSnap ? selectedSnapshot.id : null;
  const liveSnapLabel = okSnap ? formatSnapshotLabel(selectedSnapshot) : null;

  useEffect(() => {
    if (liveRepo && liveSnapId && liveSnapLabel) {
      setHeldExplorer({
        repoName: liveRepo,
        snapshotId: liveSnapId,
        snapshotLabel: liveSnapLabel,
      });
      return;
    }
    if (!loading) setHeldExplorer(null);
  }, [liveRepo, liveSnapId, liveSnapLabel, loading]);

  const showExplorer = heldExplorer !== null && !snapError && !shareStatus;
  const switching = loading && liveSnapId === null && heldExplorer !== null;
  const zeroRepos = reposReady && !configError && repos.length === 0;
  const topDirs = selectedSnapshot
    ? formatSnapshotTopDirs(selectedSnapshot.paths)
    : "";
  const emptyMsg = explorerEmptyMessage({
    configError,
    zeroRepos,
    shareStatus: !!shareStatus,
    snapError: !!snapError,
    emptyRepo:
      !loading && !snapError && !shareStatus && !!selectedRepo &&
      snapshots.length === 0,
  });
  function onShowHit(hit: SearchHit) {
    if (!snapshots.some((snapshot) => snapshot.id === hit.snapshot_id)) {
      setActionSuccess(null);
      setActionFail("This snapshot is no longer in the snapshot list.");
      return;
    }
    const req: ShowHitRequest = {
      snapshotId: hit.snapshot_id,
      path: hit.path,
      name: hit.name,
    };
    setShowHitRequest(req);
    if (hit.snapshot_id !== heldExplorer?.snapshotId) {
      selectSnapshotId(hit.snapshot_id);
    }
  }

  const remapSearchSnapshotId = useRef<(oldId: string, newId: string) => void>(
    () => {},
  );
  const onAfterTagSave = useCallback(
    async (change: { old_snapshot_id: string; new_snapshot_id: string }) => {
      clearListDirCacheForRepo(selectedRepo);
      remapSearchSnapshotId.current(
        change.old_snapshot_id,
        change.new_snapshot_id,
      );
      await refreshSelecting(change.new_snapshot_id);
    },
    [selectedRepo, refreshSelecting],
  );

  const tags = useSnapshotTags({
    repoName: selectedRepo,
    snapshotId: selectedSnapshot?.id ?? null,
    currentTags: selectedSnapshot?.tags ?? [],
    actionBusy,
    onActionId: setActionId,
    onBusyLabel: setBusyLabel,
    onAfterSave: onAfterTagSave,
  });
  const chromeLocked = actionBusy || overwriteOpen || tags.open;

  const search = useSearchAppWire({
    enabled: showExplorer && !switching,
    repoName: heldExplorer?.repoName ?? selectedRepo,
    snapshotId: heldExplorer?.snapshotId ?? null,
    snapshots,
    actionBusy,
    overwriteOpen,
    uiBlocked: tags.open,
    cancellation,
    onActionId: setActionId,
    onBusyLabel: setBusyLabel,
    onOverwriteOpenChange: setOverwriteOpen,
    onSuccess: (m) => {
      setActionSuccess(m);
      setActionFail(null);
    },
    onError: (m) => {
      setActionFail(m);
      setActionSuccess(null);
    },
    onShowHit,
  });
  remapSearchSnapshotId.current = search.remapSnapshotId;

  return (
    <main className="app">
      <header className="explorer-chrome" inert={search.open || tags.open}>
        <div className="app-brand">
          <ArchiveMark />
          <span className="app-brand-name">restarilo</span>
        </div>
        <div className="app-chrome-center">
          <div className="app-chrome-paths" title={selectedSnapshot?.paths?.join("\n")} aria-live="polite">
            {topDirs || "\u00a0"}
          </div>
          {okSnap && (
            <SnapshotTagChips
              tags={selectedSnapshot.tags}
              disabled={chromeLocked && !tags.open}
              onOpenEditor={tags.openEditor}
            />
          )}
        </div>
        <div className="app-chrome-controls">
          <ExplorerChromeControls
            configError={configError} zeroRepos={zeroRepos} repos={repos}
            selectedRepo={selectedRepo} onSelectRepo={onSelectRepo}
            chromeLocked={chromeLocked} snapshots={snapshots}
            selectedSnapshotId={selectedSnapshot?.id ?? null} loading={loading}
            snapError={snapError} onSelectSnapshotId={selectSnapshotId}
            onRetryList={retryList} shareStatus={shareStatus}
            onSignIn={signIn} onTryAnotherAccount={tryAnotherAccount}
          />
        </div>
      </header>
      {search.overlay}
      <SnapshotTagEditor
        open={tags.open}
        initialTags={selectedSnapshot?.tags ?? []}
        saving={tags.saving}
        error={tags.error}
        onSave={(draft) => void tags.save(draft)}
        onCancel={tags.closeEditor}
      />
      {actionBusy && !search.searching && !tags.saving && (
        <BusyModal label={busyLabel ?? "Working…"} {...cancellation} />
      )}
      <section className="explorer" inert={search.open || tags.open}>
        {!search.open && !tags.open && actionSuccess && (
          <p className="app-success app-action-feedback">{actionSuccess}</p>
        )}
        {!search.open && !tags.open && actionFail && (
          <pre className="app-error app-action-feedback">{actionFail}</pre>
        )}
        {showExplorer && heldExplorer ? (
          <div className={switching ? "explorer-hold is-switching" : "explorer-hold"}>
            {switching && (
              <p className="app-loading explorer-switch-note" role="status">Loading snapshots…</p>
            )}
            <FileBrowser
              repoName={heldExplorer.repoName} snapshotId={heldExplorer.snapshotId}
              snapshotLabel={heldExplorer.snapshotLabel}
              actionBusy={actionBusy || switching} overwriteOpen={overwriteOpen}
              showHitRequest={showHitRequest}
              onShowHitConsumed={() => setShowHitRequest(null)}
              onOpenSearch={() => search.setOpen(true)}
              onActionId={setActionId} onBusyLabel={setBusyLabel}
              onOverwriteOpenChange={setOverwriteOpen}
            />
          </div>
        ) : (
          <div className="explorer-listing"><p className="app-loading">{emptyMsg}</p></div>
        )}
      </section>
    </main>
  );
}

export default App;
