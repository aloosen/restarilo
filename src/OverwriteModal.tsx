type Props = {
  sourcePath: string;
  snapshotLabel: string;
  targetPath: string;
  onConfirm: () => void;
  onCancel: () => void;
};

export function OverwriteModal({
  sourcePath,
  snapshotLabel,
  targetPath,
  onConfirm,
  onCancel,
}: Props) {
  return (
    <div className="busy-dimmer" role="presentation" onClick={onCancel}>
      <div
        className="busy-modal overwrite-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="overwrite-modal-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h2 id="overwrite-modal-title" className="busy-modal-label">
          Overwrite?
        </h2>
        <p className="overwrite-modal-lead">Target already exists.</p>
        <dl className="overwrite-modal-dl">
          <dt>Source</dt>
          <dd>{sourcePath}</dd>
          <dt>Snapshot</dt>
          <dd>{snapshotLabel}</dd>
          <dt>Destination</dt>
          <dd>{targetPath}</dd>
        </dl>
        <div className="overwrite-modal-actions">
          <button type="button" className="overwrite-modal-secondary" onClick={onCancel}>
            Cancel
          </button>
          <button
            type="button"
            className="busy-modal-cancel"
            onClick={onConfirm}
            autoFocus
          >
            Overwrite
          </button>
        </div>
      </div>
    </div>
  );
}
