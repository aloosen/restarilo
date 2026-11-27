import { useEffect, useRef, type RefObject } from "react";
import "./SearchOverlay.css";

export type SearchMode = "current" | "all";

export type SearchHit = {
  snapshot_id: string;
  path: string;
  name: string;
  is_dir: boolean;
};

type Props = {
  open: boolean;
  mode: SearchMode;
  query: string;
  covered: boolean;
  canSearch: boolean;
  input: RefObject<HTMLInputElement | null>;
  matches: SearchHit[] | null;
  truncated: boolean;
  showSnapshotLabels: boolean;
  error: string | null;
  notice: string | null;
  searching: boolean;
  cancelled: boolean;
  canCancel: boolean;
  cancelling: boolean;
  cancelError?: string;
  cancel: () => void;
  labelForSnapshot: (id: string) => string;
  onClose: () => void;
  onModeChange: (mode: SearchMode) => void;
  onQueryChange: (query: string) => void;
  onSearch: () => void;
  onShowHit?: (hit: SearchHit) => void;
  onExportHit?: (hit: SearchHit) => void;
};

export function SearchOverlay(p: Props) {
  const cancelButton = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!p.open || p.covered) return;
    if (p.canCancel) cancelButton.current?.focus();
    else if (!p.searching) p.input.current?.focus();
  }, [p.open, p.canCancel, p.searching, p.covered, p.input]);
  if (!p.open) return null;

  return (
    <div className="search-dimmer" role="presentation">
      <div
        className="search-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="search-panel-title"
        inert={p.covered}
      >
        <div className="search-panel-head">
          <h2 id="search-panel-title">Search</h2>
          <button
            type="button"
            className="search-close"
            onClick={p.onClose}
            disabled={p.searching}
          >
            Close
          </button>
        </div>
        <div className="search-mode" role="group" aria-label="Search scope">
          <label>
            <input
              type="radio"
              name="search-mode"
              checked={p.mode === "current"}
              disabled={p.searching}
              onChange={() => p.onModeChange("current")}
            />{" "}
            Current snapshot
          </label>
          <label>
            <input
              type="radio"
              name="search-mode"
              checked={p.mode === "all"}
              disabled={p.searching}
              onChange={() => p.onModeChange("all")}
            />{" "}
            All snapshots
          </label>
        </div>
        <form className="search-row" onSubmit={(event) => {
          event.preventDefault();
          if (p.canSearch) p.onSearch();
        }}>
          <input
            ref={p.input}
            type="search"
            className="search-input"
            value={p.query}
            placeholder="File or path…"
            aria-label="File or path"
            disabled={p.searching}
            onChange={(e) => p.onQueryChange(e.target.value)}
          />
          {p.searching ? (
            <button type="button" className="search-close" ref={cancelButton}
              onClick={p.cancel} disabled={!p.canCancel || p.cancelling}>
              Cancel
            </button>
          ) : (
            <button type="submit" className="search-submit" disabled={!p.canSearch}>Search</button>
          )}
        </form>
        {p.searching && (
          <div className="search-progress" role="status">
            <span className="search-spinner" aria-hidden="true" />
            <div>
              <strong>{p.cancelling ? "Cancelling search…" :
                p.mode === "all" ? "Searching all snapshots…" : "Searching current snapshot…"}</strong>
              {!p.cancelling && p.mode === "all" && (
                <p className="search-status">This can take longer on remote repositories.</p>
              )}
            </div>
          </div>
        )}
        {p.searching && p.cancelError && <p className="search-error" role="alert">{p.cancelError}</p>}
        <div className="search-results" aria-live="polite" aria-busy={p.searching}>
          {p.cancelled && <p className="search-status">Search cancelled.</p>}
          {!p.searching && p.notice && (
            <p className="search-notice app-success">{p.notice}</p>
          )}
          {!p.searching && p.error && (
            <p className="search-error" role="alert">
              {p.error}
            </p>
          )}
          {!p.searching && p.matches && p.matches.length === 0 && (
            <p className="search-status">No matches</p>
          )}
          {!p.searching && p.matches && p.matches.length > 0 && (
            <>
              {p.truncated && (
                <p className="search-status">
                  Too many matches — refine your query
                </p>
              )}
              <ul className="search-hit-list">
                {p.matches.map((hit) => (
                  <li
                    key={`${hit.snapshot_id}\0${hit.path}`}
                    className="search-hit"
                  >
                    <div className="search-hit-main">
                      <span className="search-hit-path" title={hit.path}>
                        {hit.path}
                      </span>
                      {p.showSnapshotLabels && (
                        <span className="search-hit-snap">
                          {p.labelForSnapshot(hit.snapshot_id)}
                        </span>
                      )}
                    </div>
                    <div className="search-hit-actions">
                      <button
                        type="button"
                        disabled={!p.onShowHit || p.covered}
                        onClick={() => p.onShowHit?.(hit)}
                      >
                        Show
                      </button>
                      <button
                        type="button"
                        disabled={!p.onExportHit || p.covered}
                        onClick={() => p.onExportHit?.(hit)}
                      >
                        Export
                      </button>
                    </div>
                  </li>
                ))}
              </ul>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
