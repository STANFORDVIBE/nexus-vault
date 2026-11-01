import { KeyRoundIcon, StickyNoteIcon } from "lucide-react";
import { cn } from "cn";
import type { ItemType } from "@/types/vault";

// Deterministic tint from the title, so an item keeps its color between sessions.
// No favicons: fetching them would leak the list of sites to the network.
const TINTS = [
  "bg-[#3b4a63] text-[#c8d6f0]",
  "bg-[#4a3b5c] text-[#dccaf0]",
  "bg-[#3b5a4f] text-[#c5ecd9]",
  "bg-[#5c4a36] text-[#f0dcc0]",
  "bg-[#5a3b44] text-[#f0c8d2]",
  "bg-[#3b5459] text-[#c3e6eb]",
];

function tintFor(text: string): string {
  let hash = 0;
  for (const char of text) hash = (hash * 31 + char.charCodeAt(0)) | 0;
  return TINTS[Math.abs(hash) % TINTS.length] ?? TINTS[0]!;
}

interface ItemAvatarProps {
  type: ItemType;
  title: string;
  size?: "sm" | "lg";
  className?: string;
}

export function ItemAvatar({ type, title, size = "sm", className }: ItemAvatarProps) {
  const box = size === "lg" ? "size-12 rounded-lg text-lg" : "size-9 rounded-md text-sm";
  if (type === "secure_note") {
    return (
      <div
        aria-hidden="true"
        className={cn(
          "flex shrink-0 items-center justify-center bg-muted text-muted-foreground",
          box,
          className,
        )}
      >
        <StickyNoteIcon className={size === "lg" ? "size-5" : "size-4"} strokeWidth={1.75} />
      </div>
    );
  }
  const letter = [...title.trim()][0]?.toUpperCase();
  return (
    <div
      aria-hidden="true"
      className={cn(
        "flex shrink-0 items-center justify-center font-semibold",
        tintFor(title),
        box,
        className,
      )}
    >
      {letter ?? <KeyRoundIcon className="size-4" />}
    </div>
  );
}
