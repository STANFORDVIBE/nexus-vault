import ReactMarkdown, { type Components } from "react-markdown";
import { toast } from "sonner";
import { errorMessage } from "@/services/errors";
import { openExternalUrl } from "@/services/vault";

// Links never navigate the WebView: they open in the system browser through Rust,
// which allows only http(s). Images are not rendered at all (no network requests).
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
  img: ({ alt }) => <span className="text-muted-foreground">[{alt || "изображение"}]</span>,
};

export function Markdown({ children }: { children: string }) {
  return (
    <div className="markdown text-sm leading-relaxed">
      <ReactMarkdown components={components} skipHtml>
        {children}
      </ReactMarkdown>
    </div>
  );
}
