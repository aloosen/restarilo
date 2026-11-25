import { icons, TablerSvg } from "./tablerIcons";

type Props = {
  disabled: boolean;
  openDisabled: boolean;
  onRestore: () => void;
  onExport: () => void;
  onOpen: () => void;
};

export function FileActionButtons(p: Props) {
  return (
    <div className="file-actions">
      <button
        type="button"
        className="file-actions-primary"
        disabled={p.disabled}
        onClick={p.onRestore}
      >
        <TablerSvg svg={icons.restore} className="action-icon" />
        Restore
      </button>
      <button type="button" disabled={p.disabled} onClick={p.onExport}>
        <TablerSvg svg={icons.export} className="action-icon" />
        Export
      </button>
      <button type="button" disabled={p.openDisabled} onClick={p.onOpen}>
        <TablerSvg svg={icons.open} className="action-icon" />
        Open
      </button>
    </div>
  );
}
