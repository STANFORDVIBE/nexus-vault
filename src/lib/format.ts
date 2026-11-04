import type { ImportSummary } from "@/types/vault";

const relative = new Intl.RelativeTimeFormat("ru", { numeric: "auto" });
const dateTime = new Intl.DateTimeFormat("ru", { dateStyle: "medium", timeStyle: "short" });
const dateOnly = new Intl.DateTimeFormat("ru", { day: "numeric", month: "short" });

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

/** "только что", "5 минут назад", "вчера", then a short date. */
export function formatRelative(millis: number, now = Date.now()): string {
  const diff = now - millis;
  if (diff < MINUTE) return "только что";
  if (diff < HOUR) return relative.format(-Math.floor(diff / MINUTE), "minute");
  if (diff < DAY) return relative.format(-Math.floor(diff / HOUR), "hour");
  if (diff < 7 * DAY) return relative.format(-Math.floor(diff / DAY), "day");
  return dateOnly.format(millis);
}

export function formatDateTime(millis: number): string {
  return dateTime.format(millis);
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} Б`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} КБ`;
  return `${(bytes / 1024 / 1024).toFixed(1)} МБ`;
}

/** Russian plural: plural(5, ["запись", "записи", "записей"]) → "записей". */
export function plural(n: number, forms: readonly [string, string, string]): string {
  const mod10 = n % 10;
  const mod100 = n % 100;
  if (mod10 === 1 && mod100 !== 11) return forms[0];
  if (mod10 >= 2 && mod10 <= 4 && (mod100 < 12 || mod100 > 14)) return forms[1];
  return forms[2];
}

export const ITEMS_FORMS = ["запись", "записи", "записей"] as const;

/** `123456` -> `123 456`, the way authenticator apps show codes. */
export function groupCode(code: string): string {
  const half = Math.ceil(code.length / 2);
  return `${code.slice(0, half)} ${code.slice(half)}`;
}

/** "Импортировано 12 записей (Bitwarden). Уже были: 3." */
export function importMessage(s: ImportSummary): { title: string; description?: string } {
  const title =
    s.imported > 0
      ? `Импортировано: ${s.imported} ${plural(s.imported, ["запись", "записи", "записей"])}`
      : "Новых записей нет";
  const notes = [`Формат: ${s.format}`];
  if (s.duplicates > 0) notes.push(`уже были в хранилище: ${s.duplicates}`);
  if (s.skipped > 0) notes.push(`не удалось прочитать: ${s.skipped}`);
  return { title, description: notes.join(", ") };
}
