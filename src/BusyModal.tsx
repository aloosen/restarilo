type Props = {
  label: string;
  onCancel: () => void;
};

export function BusyModal({ label, onCancel }: Props) {
  return (
    <div className="busy-dimmer" role="presentation">
      <div
        className="busy-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="busy-modal-title"
      >
        <p id="busy-modal-title" className="busy-modal-label">
          {label}
        </p>
        <button type="button" className="busy-modal-cancel" onClick={onCancel} autoFocus>
          Cancel
        </button>
        <p className="busy-modal-hint">Esc cancels</p>
      </div>
    </div>
  );
}
