import { useState } from "react";
import ReactMarkdown, { defaultUrlTransform, type Components } from "react-markdown";
import { DownloadIcon, ImageOffIcon } from "lucide-react";
import { toast } from "sonner";
import { cn } from "cn";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog";
import { ATTACHMENT_PREFIX, useAttachmentUrl } from "@/features/items/attachments";
import { keepBackslashes } from "@/features/items/noteDraft";
import { errorMessage } from "@/services/errors";
import { openExternalUrl, saveAttachment } from "@/services/vault";

// Links never navigate the WebView: they open in the system browser through Rust,
// which allows only http(s). Only the note's own encrypted images are shown;
// external image URLs are never loaded (no network requests).
const components: Components = {
  a: ({ href, children }) => (
    <a
      href={href}
      onClick={(event) => {
        event.preventDefault();
        if (href) openExternalUrl(href).catch((e: unknown) => toast.error(errorMessage(e)));
      }}
      className="text-brand underline underline-offset-3 hover:text-brand/80"
    >
      {children}
    </a>
  ),
  img: ({ src, alt }) =>
    typeof src === "string" && src.startsWith(ATTACHMENT_PREFIX) ? (
      <AttachmentImage id={src.slice(ATTACHMENT_PREFIX.length)} alt={alt ?? ""} />
    ) : (
      <span className="text-muted-foreground">[{alt || "изображение"}]</span>
    ),
};

const urlTransform = (url: string) =>
  url.startsWith(ATTACHMENT_PREFIX) ? url : defaultUrlTransform(url);

export function Markdown({ children }: { children: string }) {
  return (
    <div className="markdown text-sm leading-relaxed">
      <ReactMarkdown components={components} urlTransform={urlTransform} skipHtml>
        {keepBackslashes(children)}
      </ReactMarkdown>
    </div>
  );
}

interface AttachmentImageProps {
  id: string;
  alt: string;
  /** Square thumbnail instead of the full-width picture. */
  thumbnail?: boolean;
}

/** An encrypted note image; click opens it full size. */
export function AttachmentImage({ id, alt, thumbnail }: AttachmentImageProps) {
  const { url, failed } = useAttachmentUrl(id);
  const [open, setOpen] = useState(false);

  async function save() {
    try {
      const name = await saveAttachment(id);
      if (name) toast.success("Картинка сохранена", { description: name });
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  if (failed) {
    return (
      <span className="inline-flex items-center gap-1.5 rounded-md border border-dashed border-border px-2 py-1 text-xs text-muted-foreground">
        <ImageOffIcon className="size-3.5" />
        Картинка не найдена
      </span>
    );
  }
  if (!url) {
    return (
      <span
        role="img"
        aria-label={alt || "Картинка загружается"}
        className={cn(
          "inline-block animate-pulse rounded-md bg-muted align-middle",
          thumbnail ? "size-20" : "h-40 w-64 max-w-full",
        )}
      />
    );
  }
  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className={cn(
          "cursor-zoom-in overflow-hidden rounded-md border border-border align-middle focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:outline-none",
          thumbnail ? "inline-block size-20" : "my-1 block max-w-full",
        )}
        aria-label={alt ? `Открыть картинку «${alt}»` : "Открыть картинку"}
      >
        <img
          src={url}
          alt={alt}
          draggable={false}
          className={thumbnail ? "size-full object-cover" : "max-h-[28rem] max-w-full"}
        />
      </button>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="w-auto max-w-[95vw] p-2 sm:max-w-[95vw]">
          <DialogTitle className="sr-only">{alt || "Картинка"}</DialogTitle>
          <img
            src={url}
            alt={alt}
            className="max-h-[82vh] max-w-[calc(95vw-1rem)] rounded-md object-contain"
          />
          <div className="flex justify-end">
            <Button type="button" variant="outline" size="sm" onClick={() => void save()}>
              <DownloadIcon />
              Сохранить в файл
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
