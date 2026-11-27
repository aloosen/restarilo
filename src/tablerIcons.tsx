import type { FileKind } from "./fileKind";
import archive from "./icons/tabler/archive.svg?raw";
import arrowBackUp from "./icons/tabler/arrow-back-up.svg?raw";
import arrowBarToDown from "./icons/tabler/arrow-bar-to-down.svg?raw";
import chevronDown from "./icons/tabler/chevron-down.svg?raw";
import chevronRight from "./icons/tabler/chevron-right.svg?raw";
import code from "./icons/tabler/code.svg?raw";
import database from "./icons/tabler/database.svg?raw";
import deviceDesktop from "./icons/tabler/device-desktop.svg?raw";
import externalLink from "./icons/tabler/external-link.svg?raw";
import file from "./icons/tabler/file.svg?raw";
import fileSpreadsheet from "./icons/tabler/file-spreadsheet.svg?raw";
import fileText from "./icons/tabler/file-text.svg?raw";
import fileTypeCsv from "./icons/tabler/file-type-csv.svg?raw";
import fileTypeDoc from "./icons/tabler/file-type-doc.svg?raw";
import fileTypeDocx from "./icons/tabler/file-type-docx.svg?raw";
import fileTypePdf from "./icons/tabler/file-type-pdf.svg?raw";
import fileTypePpt from "./icons/tabler/file-type-ppt.svg?raw";
import fileTypeXls from "./icons/tabler/file-type-xls.svg?raw";
import fileZip from "./icons/tabler/file-zip.svg?raw";
import folder from "./icons/tabler/folder.svg?raw";
import link from "./icons/tabler/link.svg?raw";
import music from "./icons/tabler/music.svg?raw";
import photo from "./icons/tabler/photo.svg?raw";
import presentation from "./icons/tabler/presentation.svg?raw";
import search from "./icons/tabler/search.svg?raw";
import video from "./icons/tabler/video.svg?raw";

const KIND_SVG: Record<FileKind, string> = {
  folder,
  symlink: link,
  pdf: fileTypePdf,
  docx: fileTypeDocx,
  doc: fileTypeDoc,
  csv: fileTypeCsv,
  zip: fileZip,
  xls: fileTypeXls,
  ppt: fileTypePpt,
  document: fileText,
  spreadsheet: fileSpreadsheet,
  presentation,
  image: photo,
  archive,
  code,
  audio: music,
  video,
  database,
  file,
};

type Props = { svg: string; className?: string };

export function TablerSvg({ svg, className }: Props) {
  return (
    <span
      className={className ? `tabler-icon ${className}` : "tabler-icon"}
      aria-hidden="true"
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  );
}

export function EntryIcon({
  kind,
  className,
}: {
  kind: FileKind;
  className?: string;
}) {
  const cls =
    kind === "folder"
      ? `listing-icon listing-icon-folder${className ? ` ${className}` : ""}`
      : `listing-icon${className ? ` ${className}` : ""}`;
  return <TablerSvg svg={KIND_SVG[kind]} className={cls} />;
}

export const icons = {
  folder,
  drive: deviceDesktop,
  chevronRight,
  chevronDown,
  restore: arrowBackUp,
  export: arrowBarToDown,
  open: externalLink,
  search,
};
