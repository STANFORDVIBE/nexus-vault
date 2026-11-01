export interface NoteDraft {
  title: string;
  content: string;
  tags: string[];
  folderId: string | null;
  favorite: boolean;
}

export const EMPTY_NOTE: NoteDraft = {
  title: "",
  content: "",
  tags: [],
  folderId: null,
  favorite: false,
};

export type Edit =
  | { kind: "wrap"; before: string; after: string; placeholder: string }
  | { kind: "prefix"; prefix: string };

/** Applies a toolbar edit to the textarea selection; returns the new text and selection. */
export function applyEdit(text: string, start: number, end: number, edit: Edit) {
  if (edit.kind === "wrap") {
    const selected = text.slice(start, end) || edit.placeholder;
    const next = text.slice(0, start) + edit.before + selected + edit.after + text.slice(end);
    const selStart = start + edit.before.length;
    return { text: next, start: selStart, end: selStart + selected.length };
  }
  const lineStart = text.lastIndexOf("\n", start - 1) + 1;
  const block = text.slice(lineStart, end);
  const prefixed = block
    .split("\n")
    .map((line) => edit.prefix + line)
    .join("\n");
  const next = text.slice(0, lineStart) + prefixed + text.slice(end);
  return { text: next, start: lineStart + prefixed.length, end: lineStart + prefixed.length };
}
