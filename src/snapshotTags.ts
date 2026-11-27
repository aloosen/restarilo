export function normalizeTagDraft(tags: string[]): {
  tags: string[];
  error: string | null;
} {
  const out: string[] = [];
  for (const raw of tags) {
    const tag = raw.trim();
    if (!tag) return { tags: out, error: "Tag must not be empty." };
    if (tag.includes(",")) {
      return { tags: out, error: "Tag must not contain a comma." };
    }
    if (out.includes(tag)) {
      return { tags: out, error: `Duplicate tag “${tag}”.` };
    }
    out.push(tag);
  }
  return { tags: out, error: null };
}

export function tagsEqual(a: string[], b: string[]): boolean {
  if (a.length !== b.length) return false;
  return a.every((t, i) => t === b[i]);
}

export function headerTagChips(tags: string[]): {
  shown: string[];
  extra: number;
} {
  if (tags.length <= 2) return { shown: tags, extra: 0 };
  return { shown: tags.slice(0, 2), extra: tags.length - 2 };
}
