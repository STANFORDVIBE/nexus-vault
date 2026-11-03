import { useCallback, useEffect, useRef, useState } from "react";
import {
  BoldIcon,
  CheckIcon,
  CodeIcon,
  HeadingIcon,
  ImageIcon,
  ItalicIcon,
  LinkIcon,
  ListChecksIcon,
  ListIcon,
  StarIcon,
  TextQuoteIcon,
} from "lucide-react";
import { toast } from "sonner";
import { cn } from "cn";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { isImageFile, referencedAttachments, uploadImage } from "@/features/items/attachments";
import { FolderSelect } from "@/features/items/FolderSelect";
import { AttachmentImage, Markdown } from "@/features/items/Markdown";
import { applyEdit, type Edit, type NoteDraft } from "@/features/items/noteDraft";
import { TagInput } from "@/features/items/TagInput";
import { errorMessage } from "@/services/errors";
import { createItem, setFavorite, updateItem } from "@/services/vault";
import { invalidate } from "@/store/session";

const MAX_CONTENT = 1_000_000;
const AUTOSAVE_MS = 700;

type SaveState = "idle" | "saving" | "saved" | "error";

const TOOLBAR: { label: string; icon: typeof BoldIcon; edit: Edit }[] = [
  {
    label: "Жирный",
    icon: BoldIcon,
    edit: { kind: "wrap", before: "**", after: "**", placeholder: "текст" },
  },
  {
    label: "Курсив",
    icon: ItalicIcon,
    edit: { kind: "wrap", before: "*", after: "*", placeholder: "текст" },
  },
  { label: "Заголовок", icon: HeadingIcon, edit: { kind: "prefix", prefix: "## " } },
  { label: "Список", icon: ListIcon, edit: { kind: "prefix", prefix: "- " } },
  { label: "Чек-лист", icon: ListChecksIcon, edit: { kind: "prefix", prefix: "- [ ] " } },
  { label: "Цитата", icon: TextQuoteIcon, edit: { kind: "prefix", prefix: "> " } },
  {
    label: "Код",
    icon: CodeIcon,
    edit: { kind: "wrap", before: "`", after: "`", placeholder: "код" },
  },
  {
    label: "Ссылка",
    icon: LinkIcon,
    edit: { kind: "wrap", before: "[", after: "](https://)", placeholder: "текст ссылки" },
  },
];

interface NoteEditorProps {
  itemId?: string;
  initial: NoteDraft;
  /** Called with the saved id, or `null` if a new note was closed before it got a title. */
  onDone: (id: string | null) => void;
}

/** Markdown note editor with autosave: changes are encrypted and written as you type. */
export function NoteEditor({ itemId, initial, onDone }: NoteEditorProps) {
  const [draft, setDraft] = useState(initial);
  const [mode, setMode] = useState<"edit" | "preview">("edit");
  const [saveState, setSaveState] = useState<SaveState>("idle");
  const [uploading, setUploading] = useState(0);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const idRef = useRef<string | undefined>(itemId);
  const savedRef = useRef({ snapshot: JSON.stringify(initial), favorite: initial.favorite });
  const queueRef = useRef<Promise<void>>(Promise.resolve());

  const save = useCallback((next: NoteDraft) => {
    // serialize saves so a slow create cannot race with the next update
    queueRef.current = queueRef.current.then(async () => {
      const snapshot = JSON.stringify(next);
      if (snapshot === savedRef.current.snapshot || !next.title.trim()) return;
      setSaveState("saving");
      try {
        const input = {
          type: "secure_note" as const,
          payload: { title: next.title.trim(), content: next.content, tags: next.tags },
          folderId: next.folderId,
          favorite: next.favorite,
        };
        if (idRef.current) {
          await updateItem(idRef.current, input);
          if (next.favorite !== savedRef.current.favorite) {
            await setFavorite(idRef.current, next.favorite);
          }
        } else {
          idRef.current = await createItem(input);
        }
        savedRef.current = { snapshot, favorite: next.favorite };
        setSaveState("saved");
        invalidate();
      } catch (e) {
        setSaveState("error");
        toast.error(errorMessage(e));
      }
    });
    return queueRef.current;
  }, []);

  useEffect(() => {
    const timer = window.setTimeout(() => void save(draft), AUTOSAVE_MS);
    return () => window.clearTimeout(timer);
  }, [draft, save]);

  const done = useCallback(async () => {
    await save(draft);
    onDone(idRef.current ?? null);
  }, [draft, save, onDone]);

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      const layerOpen = document.querySelector(
        "[role=dialog],[role=alertdialog],[data-radix-popper-content-wrapper]",
      );
      if (event.key === "Escape" && !layerOpen) {
        event.preventDefault();
        void done();
      } else if ((event.ctrlKey || event.metaKey) && event.code === "KeyS") {
        event.preventDefault();
        void save(draft);
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [done, save, draft]);

  const patch = (change: Partial<NoteDraft>) => setDraft((d) => ({ ...d, ...change }));

  function runEdit(edit: Edit) {
    const el = textareaRef.current;
    if (!el) return;
    const result = applyEdit(draft.content, el.selectionStart, el.selectionEnd, edit);
    patch({ content: result.text.slice(0, MAX_CONTENT) });
    requestAnimationFrame(() => {
      el.focus();
      el.setSelectionRange(result.start, result.end);
    });
  }

  /** Images belong to a saved note, so a brand-new one is saved first. */
  async function ensureSaved(): Promise<string> {
    if (!idRef.current) {
      const next = { ...draft, title: draft.title.trim() || "Без названия" };
      setDraft(next);
      await save(next);
    }
    if (!idRef.current) throw new Error("not saved");
    return idRef.current;
  }

  async function insertImages(files: File[]) {
    const images = files.filter(isImageFile);
    if (images.length === 0) {
      if (files.length > 0) toast.error("Поддерживаются картинки PNG, JPEG, WebP и GIF");
      return;
    }
    const el = textareaRef.current;
    const at = el ? el.selectionEnd : draft.content.length;
    setUploading((n) => n + 1);
    try {
      const itemId = await ensureSaved();
      const parts: string[] = [];
      for (const file of images) {
        try {
          parts.push(await uploadImage(itemId, file));
        } catch (e) {
          toast.error(file.name ? `${file.name}: ${errorMessage(e)}` : errorMessage(e));
        }
      }
      if (parts.length === 0) return;
      setDraft((d) => {
        const pos = Math.min(at, d.content.length);
        const before = d.content.slice(0, pos);
        const after = d.content.slice(pos);
        const NL = "\n";
        const block =
          (before && !before.endsWith(NL) ? NL : "") +
          parts.join(NL) +
          (after.startsWith(NL) ? "" : NL);
        return { ...d, content: (before + block + after).slice(0, MAX_CONTENT) };
      });
    } catch {
      // the save error was already shown
    } finally {
      setUploading((n) => n - 1);
    }
  }

  function onPaste(event: React.ClipboardEvent<HTMLTextAreaElement>) {
    const files = [...event.clipboardData.files];
    if (files.some(isImageFile)) {
      event.preventDefault();
      void insertImages(files);
    }
  }

  function onDrop(event: React.DragEvent<HTMLTextAreaElement>) {
    const files = [...event.dataTransfer.files];
    if (files.length > 0) {
      event.preventDefault();
      void insertImages(files);
    }
  }

  const images = referencedAttachments(draft.content);

  const statusText = uploading
    ? "Добавляем картинку…"
    : !draft.title.trim()
      ? "Укажите название, чтобы сохранить"
      : saveState === "saving"
        ? "Сохраняем…"
        : saveState === "error"
          ? "Не сохранено"
          : saveState === "saved"
            ? "Сохранено"
            : "";

  return (
    <div className="flex h-full flex-col">
      <div className="flex h-14 shrink-0 items-center justify-between gap-3 border-b border-border px-6">
        <div className="flex min-w-0 items-center gap-2">
          <h2 className="truncate text-base">{itemId ? "Заметка" : "Новая заметка"}</h2>
          <Button
            type="button"
            variant="ghost"
            size="icon-sm"
            aria-label={draft.favorite ? "Убрать из избранного" : "В избранное"}
            aria-pressed={draft.favorite}
            onClick={() => patch({ favorite: !draft.favorite })}
          >
            <StarIcon className={cn(draft.favorite && "fill-brand text-brand")} />
          </Button>
          <span
            className={cn(
              "truncate text-xs text-muted-foreground",
              saveState === "error" && "text-destructive",
            )}
            aria-live="polite"
          >
            {statusText}
          </span>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <SegmentedControl
            aria-label="Режим"
            value={mode}
            onValueChange={setMode}
            options={[
              { value: "edit", label: "Текст" },
              { value: "preview", label: "Просмотр" },
            ]}
            className="h-8"
          />
          <Button type="button" onClick={() => void done()}>
            <CheckIcon />
            Готово
          </Button>
        </div>
      </div>

      <div className="flex flex-1 flex-col gap-4 overflow-y-auto px-6 py-5">
        <Input
          value={draft.title}
          onChange={(e) => patch({ title: e.target.value.slice(0, 200) })}
          placeholder="Название"
          aria-label="Название"
          autoFocus={!itemId}
          autoComplete="off"
          className="h-10 border-none bg-transparent px-0 text-xl font-semibold shadow-none focus-visible:ring-0 dark:bg-transparent"
        />

        <div className="grid gap-4 sm:grid-cols-2">
          <div className="grid gap-2">
            <Label htmlFor="note-folder" className="text-xs text-muted-foreground">
              Папка
            </Label>
            <FolderSelect
              id="note-folder"
              value={draft.folderId}
              onChange={(folderId) => patch({ folderId })}
            />
          </div>
          <div className="grid gap-2">
            <Label htmlFor="note-tags" className="text-xs text-muted-foreground">
              Теги
            </Label>
            <TagInput id="note-tags" value={draft.tags} onChange={(tags) => patch({ tags })} />
          </div>
        </div>

        {mode === "edit" ? (
          <div className="flex min-h-80 flex-1 flex-col rounded-md border border-input focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50">
            <div
              role="toolbar"
              aria-label="Форматирование"
              className="flex flex-wrap gap-0.5 border-b border-border px-1.5 py-1"
            >
              {TOOLBAR.map(({ label, icon: Icon, edit }) => (
                <Tooltip key={label}>
                  <TooltipTrigger asChild>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      aria-label={label}
                      onClick={() => runEdit(edit)}
                      className="text-muted-foreground hover:text-foreground"
                    >
                      <Icon />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{label}</TooltipContent>
                </Tooltip>
              ))}
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    aria-label="Картинка"
                    onClick={() => fileInputRef.current?.click()}
                    className="text-muted-foreground hover:text-foreground"
                  >
                    <ImageIcon />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>Картинка — или вставьте скриншот через Ctrl+V</TooltipContent>
              </Tooltip>
              <input
                ref={fileInputRef}
                type="file"
                accept="image/png,image/jpeg,image/webp,image/gif,image/bmp"
                multiple
                hidden
                onChange={(e) => {
                  void insertImages([...(e.target.files ?? [])]);
                  e.target.value = "";
                }}
              />
            </div>
            <textarea
              ref={textareaRef}
              value={draft.content}
              onChange={(e) => patch({ content: e.target.value.slice(0, MAX_CONTENT) })}
              onPaste={onPaste}
              onDrop={onDrop}
              placeholder="Текст заметки. Поддерживается Markdown: **жирный**, списки, `код`… Скриншот — Ctrl+V"
              aria-label="Текст заметки"
              autoFocus={!!itemId}
              spellCheck={false}
              className="flex-1 resize-none bg-transparent px-3 py-2.5 font-mono text-sm leading-relaxed outline-none placeholder:text-muted-foreground"
            />
            {images.length > 0 && (
              <div
                aria-label="Картинки в заметке"
                className="flex flex-wrap gap-2 border-t border-border px-3 py-2"
              >
                {images.map((id) => (
                  <AttachmentImage key={id} id={id} alt="" thumbnail />
                ))}
              </div>
            )}
          </div>
        ) : (
          <div className="min-h-80 flex-1 rounded-md border border-border px-4 py-3">
            {draft.content.trim() ? (
              <Markdown>{draft.content}</Markdown>
            ) : (
              <p className="text-sm text-muted-foreground">Пустая заметка</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
