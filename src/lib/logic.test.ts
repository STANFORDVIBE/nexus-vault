import { applyEdit } from "@/features/items/noteDraft";
import { newItemBase } from "@/hooks/useAppHotkeys";
import { formatBytes, plural, ITEMS_FORMS } from "@/lib/format";
import { estimateStrength } from "@/lib/password-strength";
import { errorMessage } from "@/services/errors";

describe("plural", () => {
  it.each([
    [1, "запись"],
    [2, "записи"],
    [5, "записей"],
    [11, "записей"],
    [21, "запись"],
    [104, "записи"],
    [112, "записей"],
  ])("%i → %s", (n, form) => expect(plural(n, ITEMS_FORMS)).toBe(form));
});

describe("formatBytes", () => {
  it("picks a unit", () => {
    expect(formatBytes(512)).toBe("512 Б");
    expect(formatBytes(2048)).toBe("2.0 КБ");
    expect(formatBytes(3 * 1024 * 1024)).toBe("3.0 МБ");
  });
});

describe("estimateStrength", () => {
  it("rates short, repetitive and common passwords low", () => {
    expect(estimateStrength("").level).toBe(0);
    expect(estimateStrength("abc").level).toBe(0);
    expect(estimateStrength("aaaaaaaaaaaaaaaa").level).toBeLessThanOrEqual(1);
    expect(estimateStrength("password12345").level).toBeLessThanOrEqual(1);
  });

  it("rates long mixed passphrases high", () => {
    expect(estimateStrength("Correct-Horse-Battery-Staple-42").level).toBe(4);
    expect(estimateStrength("Зелёный слон гуляет по крыше").level).toBeGreaterThanOrEqual(3);
  });
});

describe("applyEdit", () => {
  it("wraps the selection", () => {
    const r = applyEdit("hello world", 6, 11, {
      kind: "wrap",
      before: "**",
      after: "**",
      placeholder: "x",
    });
    expect(r.text).toBe("hello **world**");
    expect(r.text.slice(r.start, r.end)).toBe("world");
  });

  it("inserts a placeholder when nothing is selected", () => {
    const r = applyEdit("", 0, 0, { kind: "wrap", before: "`", after: "`", placeholder: "код" });
    expect(r.text).toBe("`код`");
    expect(r.text.slice(r.start, r.end)).toBe("код");
  });

  it("prefixes every selected line", () => {
    const r = applyEdit("a\nb\nc", 2, 5, { kind: "prefix", prefix: "- " });
    expect(r.text).toBe("a\n- b\n- c");
  });
});

describe("newItemBase", () => {
  it("opens the editor where the new item will be listed", () => {
    expect(newItemBase("/passwords", "note")).toBe("/notes");
    expect(newItemBase("/notes", "login")).toBe("/passwords");
    expect(newItemBase("/items", "note")).toBe("/items");
    expect(newItemBase("/folders/abc", "login")).toBe("/folders/abc");
    expect(newItemBase("/trash", "login")).toBe("/passwords");
    expect(newItemBase("/settings", "note")).toBe("/notes");
  });
});

describe("errorMessage", () => {
  it("translates backend error codes", () => {
    expect(errorMessage({ code: "invalid_password", message: "x" })).toBe("Неверный мастер-пароль");
    expect(errorMessage({ code: "validation", message: "validation failed: folder_exists" })).toBe(
      "Папка с таким названием уже есть",
    );
    expect(errorMessage(new Error("boom"))).toBe("Что-то пошло не так");
  });
});
