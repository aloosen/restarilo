type Props = {
  label: string;
  cancel: () => void;
  cancelling: boolean;
  cancelError?: string;
};

export function BusyModal({ label, cancel, cancelling, cancelError }: Props) {
  return (
    <div className="busy-dimmer" role="presentation">
      <div
        className="busy-modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="busy-modal-title"
      >
        <p id="busy-modal-title" className="busy-modal-label" role="status">
          {cancelling ? "Cancelling…" : label}
        </p>
        <button type="button" className="busy-modal-cancel" onClick={cancel} disabled={cancelling} autoFocus>
          Cancel
        </button>
        {cancelError && <p className="app-error" role="alert">{cancelError}</p>}
        {!cancelling && <p className="busy-modal-hint">Esc cancels</p>}
      </div>
    </div>
  );
}
