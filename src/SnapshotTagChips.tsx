import { headerTagChips } from "./snapshotTags";

type Props = {
  tags: string[];
  disabled?: boolean;
  onOpenEditor: () => void;
};

export function SnapshotTagChips({ tags, disabled, onOpenEditor }: Props) {
  const { shown, extra } = headerTagChips(tags);
  if (tags.length === 0) {
    return (
      <button
        type="button"
        className="snap-tag-add"
        disabled={disabled}
        onClick={onOpenEditor}
      >
        Add tag
      </button>
    );
  }
  return (
    <div className="snap-tag-chips" aria-label="Snapshot tags">
      {shown.map((tag) => (
        <button
          key={tag}
          type="button"
          className="snap-tag-chip"
          disabled={disabled}
          title={tag}
          onClick={onOpenEditor}
        >
          {tag}
        </button>
      ))}
      {extra > 0 && (
        <button
          type="button"
          className="snap-tag-more"
          disabled={disabled}
          onClick={onOpenEditor}
          title={`Show all ${tags.length} tags`}
        >
          +{extra}
        </button>
      )}
    </div>
  );
}
