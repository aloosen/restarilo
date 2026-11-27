import type { KeyboardEvent } from "react";
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
  actionBusy: boolean;
  matches: SearchHit[] | null;
  truncated: boolean;
  error: string | null;
  searching: boolean;
  labelForSnapshot: (id: string) => string;
  onClose: () => void;
  onModeChange: (mode: SearchMode) => void;
  onQueryChange: (query: string) => void;
  onSearch: () => void;
  onShowHit?: (hit: SearchHit) => void;
  onExportHit?: (hit: SearchHit) => void;
};

export function SearchOverlay(p: Props) {
  if (!p.open) return null;

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Enter") {
      e.preventDefault();
      if (!p.actionBusy && !p.searching) p.onSearch();
    }
  }

  const searchDisabled = p.actionBusy || p.searching || !p.query.trim();

  return (
    <div className="search-dimmer" role="presentation">
      <div
        className="search-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="search-panel-title"
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
        <div className="search-row">
          <input
            type="search"
            className="search-input"
            value={p.query}
            placeholder="File or path…"
            autoFocus
            disabled={p.searching}
            onChange={(e) => p.onQueryChange(e.target.value)}
            onKeyDown={onKeyDown}
          />
          <button
            type="button"
            className="search-submit"
            disabled={searchDisabled}
            onClick={p.onSearch}
          >
            Search
          </button>
        </div>
        <div className="search-results" aria-live="polite">
          {p.searching && <p className="search-status">Searching…</p>}
          {!p.searching && p.error && (
            <p className="search-error" role="alert">
              {p.error}
            </p>
          )}
          {!p.searching && !p.error && p.matches && p.matches.length === 0 && (
            <p className="search-status">No matches</p>
          )}
          {!p.searching && !p.error && p.matches && p.matches.length > 0 && (
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
                      {p.mode === "all" && (
                        <span className="search-hit-snap">
                          {p.labelForSnapshot(hit.snapshot_id)}
                        </span>
                      )}
                    </div>
                    <div className="search-hit-actions">
                      <button
                        type="button"
                        disabled={!p.onShowHit || p.actionBusy}
                        onClick={() => p.onShowHit?.(hit)}
                      >
                        Show
                      </button>
                      <button
                        type="button"
                        disabled={!p.onExportHit || p.actionBusy}
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
