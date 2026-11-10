import { renderToStaticMarkup } from "react-dom/server";
import ReactMarkdown from "react-markdown";
import { keepBackslashes } from "@/features/items/noteDraft";

const html = (md: string) =>
  renderToStaticMarkup(<ReactMarkdown>{keepBackslashes(md)}</ReactMarkdown>);

describe("keepBackslashes", () => {
  it("shows Windows paths and escapes exactly as typed", () => {
    expect(html("%USERPROFILE%\\.tauri\\family-budget.key")).toBe(
      "<p>%USERPROFILE%\\.tauri\\family-budget.key</p>",
    );
    expect(html("a\\\\b \\*star\\*")).toBe("<p>a\\\\b \\*star\\*</p>");
  });

  it("leaves code untouched", () => {
    const md = "`C:\\.x` and\n```\nC:\\.y\n```";
    expect(keepBackslashes(md)).toBe(md);
    expect(html("`C:\\.x`")).toBe("<p><code>C:\\.x</code></p>");
  });
});
