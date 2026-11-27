import { formatPathPreview, pathCountLabel } from "./snapshotPaths";

type Props = {
  paths: string[];
  disabled?: boolean;
  onOpenPanel: () => void;
};

export function SnapshotPathsHeader({ paths, disabled, onOpenPanel }: Props) {
  if (paths.length === 0) {
    return <div className="app-chrome-paths-row" aria-live="polite">{"\u00a0"}</div>;
  }
  const preview = formatPathPreview(paths[0]!);
  const count = pathCountLabel(paths.length);
  return (
    <div className="app-chrome-paths-row" aria-live="polite">
      <button
        type="button"
        className="app-chrome-path-preview"
        title={paths[0]}
        disabled={disabled}
        onClick={onOpenPanel}
      >
        {preview}
      </button>
      <button
        type="button"
        className="app-chrome-path-count"
        disabled={disabled}
        onClick={onOpenPanel}
        aria-label={`Show all ${count}`}
        title={`Show all ${count}`}
      >
        {count}
      </button>
    </div>
  );
}
