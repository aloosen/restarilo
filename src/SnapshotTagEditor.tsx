import { useEffect, useId, useState } from "react";
import { normalizeTagDraft } from "./snapshotTags";

type Props = {
  open: boolean;
  initialTags: string[];
  saving: boolean;
  error: string | null;
  onSave: (tags: string[]) => void;
  onCancel: () => void;
};

export function SnapshotTagEditor({
  open,
  initialTags,
  saving,
  error,
  onSave,
  onCancel,
}: Props) {
  const titleId = useId();
  const [draft, setDraft] = useState<string[]>(initialTags);
  const [input, setInput] = useState("");
  const [localError, setLocalError] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setDraft(initialTags);
    setInput("");
    setLocalError(null);
  }, [open, initialTags]);

  useEffect(() => {
    if (!open || saving) return;
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        e.preventDefault();
        onCancel();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, saving, onCancel]);

  if (!open) return null;

  function addTag() {
    const next = [...draft, input];
    const n = normalizeTagDraft(next);
    if (n.error) {
      setLocalError(n.error);
      return;
    }
    setDraft(n.tags);
    setInput("");
    setLocalError(null);
  }

  function removeTag(tag: string) {
    setDraft((d) => d.filter((t) => t !== tag));
    setLocalError(null);
  }

  function save() {
    const n = normalizeTagDraft(draft);
    if (n.error) {
      setLocalError(n.error);
      return;
    }
    if (input.trim()) {
      const withInput = normalizeTagDraft([...n.tags, input]);
      if (withInput.error) {
        setLocalError(withInput.error);
        return;
      }
      onSave(withInput.tags);
      return;
    }
    onSave(n.tags);
  }

  const displayError = localError ?? error;

  return (
    <div className="busy-dimmer snap-tag-dimmer" role="presentation">
      <div
        className="busy-modal overwrite-modal snap-tag-editor"
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id={titleId} className="busy-modal-label">
          Snapshot tags
        </h2>
        {saving ? (
          <p className="snap-tag-saving" role="status">
            Saving tags…
          </p>
        ) : (
          <>
            <ul className="snap-tag-editor-list">
              {draft.length === 0 && (
                <li className="snap-tag-editor-empty">No tags</li>
              )}
              {draft.map((tag) => (
                <li key={tag} className="snap-tag-editor-row">
                  <span className="snap-tag-editor-name">{tag}</span>
                  <button
                    type="button"
                    className="overwrite-modal-secondary"
                    onClick={() => removeTag(tag)}
                  >
                    Remove
                  </button>
                </li>
              ))}
            </ul>
            <div className="snap-tag-editor-add">
              <input
                type="text"
                value={input}
                placeholder="New tag"
                disabled={saving}
                onChange={(e) => {
                  setInput(e.target.value);
                  setLocalError(null);
                }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") {
                    e.preventDefault();
                    addTag();
                  }
                }}
                autoFocus
              />
              <button
                type="button"
                className="overwrite-modal-secondary"
                onClick={addTag}
                disabled={!input.trim()}
              >
                Add
              </button>
            </div>
          </>
        )}
        {displayError && (
          <pre className="app-error snap-tag-editor-error" role="alert">
            {displayError}
          </pre>
        )}
        <div className="overwrite-modal-actions">
          <button
            type="button"
            className="overwrite-modal-secondary"
            onClick={onCancel}
            disabled={saving}
          >
            Cancel
          </button>
          <button
            type="button"
            className="busy-modal-cancel"
            onClick={save}
            disabled={saving}
          >
            Save
          </button>
        </div>
      </div>
    </div>
  );
}
