import type { DirEntry } from "./listDirCache";

/** Visual kind for Tabler outline icons in the listing. */
export type FileKind =
  | "folder"
  | "symlink"
  | "pdf"
  | "docx"
  | "doc"
  | "csv"
  | "zip"
  | "xls"
  | "ppt"
  | "document"
  | "spreadsheet"
  | "presentation"
  | "image"
  | "archive"
  | "code"
  | "audio"
  | "video"
  | "database"
  | "file";

const COMPOUND: [string, FileKind][] = [
  [".tar.gz", "archive"],
  [".tar.bz2", "archive"],
  [".tar.xz", "archive"],
  [".tar.zst", "archive"],
];

const EXT: Record<string, FileKind> = {
  pdf: "pdf",
  docx: "docx",
  doc: "doc",
  odt: "document",
  rtf: "document",
  txt: "document",
  md: "document",
  markdown: "document",
  csv: "csv",
  tsv: "csv",
  zip: "zip",
  gz: "archive",
  bz2: "archive",
  xz: "archive",
  zst: "archive",
  "7z": "archive",
  rar: "archive",
  tar: "archive",
  tgz: "archive",
  xls: "xls",
  xlsx: "xls",
  ods: "spreadsheet",
  ppt: "ppt",
  pptx: "ppt",
  odp: "presentation",
  png: "image",
  jpg: "image",
  jpeg: "image",
  gif: "image",
  webp: "image",
  svg: "image",
  bmp: "image",
  heic: "image",
  ico: "image",
  mp3: "audio",
  wav: "audio",
  flac: "audio",
  ogg: "audio",
  m4a: "audio",
  aac: "audio",
  mp4: "video",
  mkv: "video",
  webm: "video",
  mov: "video",
  avi: "video",
  js: "code",
  ts: "code",
  tsx: "code",
  jsx: "code",
  py: "code",
  rs: "code",
  go: "code",
  java: "code",
  c: "code",
  cpp: "code",
  h: "code",
  hpp: "code",
  css: "code",
  scss: "code",
  html: "code",
  json: "code",
  toml: "code",
  yaml: "code",
  yml: "code",
  xml: "code",
  sh: "code",
  sql: "database",
  db: "database",
  sqlite: "database",
  sqlite3: "database",
};

function lowerName(name: string): string {
  return name.toLowerCase();
}

/** Resolve kind: dir/symlink first, then case-insensitive extension (compound first). */
export function resolveFileKind(entry: DirEntry): FileKind {
  const t = (entry.entry_type || "").toLowerCase();
  if (entry.is_dir || t === "dir") return "folder";
  if (t === "symlink" || t === "link") return "symlink";

  const name = lowerName(entry.name);
  for (const [suffix, kind] of COMPOUND) {
    if (name.endsWith(suffix)) return kind;
  }
  const dot = name.lastIndexOf(".");
  if (dot <= 0 || dot === name.length - 1) return "file";
  const ext = name.slice(dot + 1);
  return EXT[ext] ?? "file";
}
