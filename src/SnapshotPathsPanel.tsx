import { useEffect, useId, useState } from "react";

type Props = {
  open: boolean;
  paths: string[];
  onClose: () => void;
};

async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

export function SnapshotPathsPanel({ open, paths, onClose }: Props) {
  const titleId = useId();
  const [copied, setCopied] = useState<string | null>(null);

  useEffect(() => {
    if (!open) {
      setCopied(null);
      return;
    }
    function onKey(e: KeyboardEvent) {
      if (e.key !== "Escape") return;
      e.preventDefault();
      onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  async function onCopy(path: string) {
    const ok = await copyText(path);
    setCopied(ok ? path : null);
  }

  return (
    <div className="busy-dimmer snap-paths-dimmer" role="presentation" onClick={onClose}>
      <div
        className="busy-modal overwrite-modal snap-paths-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id={titleId} className="busy-modal-label">
          Backup paths
        </h2>
        <p className="overwrite-modal-lead">
          {paths.length === 1
            ? "1 path in this snapshot."
            : `${paths.length} paths in this snapshot.`}
        </p>
        <ul className="snap-paths-list">
          {paths.map((path) => (
            <li key={path} className="snap-paths-row">
              <code className="snap-paths-path">{path}</code>
              <button
                type="button"
                className="overwrite-modal-secondary"
                onClick={() => void onCopy(path)}
              >
                {copied === path ? "Copied" : "Copy"}
              </button>
            </li>
          ))}
        </ul>
        <div className="overwrite-modal-actions">
          <button
            type="button"
            className="busy-modal-cancel"
            onClick={onClose}
            autoFocus
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
