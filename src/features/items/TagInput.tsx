import { useState } from "react";
import { XIcon } from "lucide-react";
import { cn } from "cn";

const MAX_TAGS = 32;
const MAX_TAG_LENGTH = 64;

interface TagInputProps {
  id?: string;
  value: string[];
  onChange: (tags: string[]) => void;
  disabled?: boolean;
}

/** Chips input: Enter or comma adds a tag, Backspace on empty input removes the last one. */
export function TagInput({ id, value, onChange, disabled }: TagInputProps) {
  const [draft, setDraft] = useState("");

  function add(...raw: string[]) {
    setDraft("");
    const next = [...value];
    for (const part of raw) {
      const tag = part.trim().slice(0, MAX_TAG_LENGTH);
      if (!tag || next.length >= MAX_TAGS) continue;
      if (next.some((t) => t.toLowerCase() === tag.toLowerCase())) continue;
      next.push(tag);
    }
    if (next.length !== value.length) onChange(next);
  }

  return (
    <div
      className={cn(
        "flex min-h-9 flex-wrap items-center gap-1.5 rounded-md border border-input bg-transparent px-2 py-1.5 transition-colors focus-within:border-ring focus-within:ring-3 focus-within:ring-ring/50 dark:bg-input/30",
        disabled && "pointer-events-none opacity-50",
      )}
    >
      {value.map((tag) => (
        <span
          key={tag}
          className="inline-flex h-6 items-center gap-1 rounded-sm bg-muted pr-1 pl-2 text-xs"
        >
          {tag}
          <button
            type="button"
            onClick={() => onChange(value.filter((t) => t !== tag))}
            aria-label={`Удалить тег ${tag}`}
            className="rounded-sm text-muted-foreground hover:text-foreground"
          >
            <XIcon className="size-3" />
          </button>
        </span>
      ))}
      <input
        id={id}
        value={draft}
        disabled={disabled}
        onChange={(e) => {
          // typing or pasting "a, b, c": everything before the last comma becomes tags
          const parts = e.target.value.split(",");
          const rest = parts.pop() ?? "";
          if (parts.length) add(...parts);
          setDraft(rest);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            add(draft);
          } else if (e.key === "Backspace" && !draft && value.length) {
            onChange(value.slice(0, -1));
          }
        }}
        onBlur={() => draft && add(draft)}
        placeholder={value.length ? "" : "Добавьте тег и нажмите Enter"}
        autoComplete="off"
        spellCheck={false}
        className="h-6 min-w-24 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
      />
    </div>
  );
}
