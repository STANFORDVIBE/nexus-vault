import { z } from "zod";
import type { CardPayload, CustomField, IdentityPayload } from "@/types/vault";

export const customFieldsSchema = z
  .array(
    z.object({
      label: z.string().max(100, "Название поля — до 100 символов"),
      value: z.string().max(10_000, "Слишком длинное значение"),
      hidden: z.boolean(),
    }),
  )
  .max(50, "Не больше 50 своих полей");

/** Drops rows the user left completely empty; trims labels (values stay verbatim). */
export function cleanFields(fields: CustomField[]): CustomField[] {
  return fields
    .filter((f) => f.label.trim() || f.value)
    .map((f) => ({ ...f, label: f.label.trim() }));
}

const digits = (min: number, max: number, message: string) =>
  z
    .string()
    .trim()
    .refine((v) => {
      const d = v.replace(/[\s-]/g, "");
      return !d || (/^\d+$/.test(d) && d.length >= min && d.length <= max);
    }, message);

const common = {
  title: z.string().trim().min(1, "Укажите название").max(200, "Не длиннее 200 символов"),
  notes: z.string().max(100_000, "Слишком длинные заметки"),
  tags: z.array(z.string()).max(32),
  fields: customFieldsSchema,
  folderId: z.string().nullable(),
  favorite: z.boolean(),
};

export const cardSchema = z.object({
  ...common,
  cardholder: z.string().trim().max(512, "Слишком длинное имя"),
  number: digits(12, 19, "Номер карты — от 12 до 19 цифр"),
  expiry: z
    .string()
    .trim()
    .refine((v) => !v || /^(0?[1-9]|1[0-2])\s*\/\s*(\d{2}|\d{4})$/.test(v), "Срок в формате ММ/ГГ"),
  cvv: digits(3, 4, "CVV — 3 или 4 цифры"),
  pin: digits(4, 12, "PIN — от 4 до 12 цифр"),
});

export type CardFormValues = z.infer<typeof cardSchema>;

export const EMPTY_CARD: CardFormValues = {
  title: "",
  cardholder: "",
  number: "",
  expiry: "",
  cvv: "",
  pin: "",
  notes: "",
  tags: [],
  fields: [],
  folderId: null,
  favorite: false,
};

export const IDENTITY_KINDS = [
  { value: "passport", label: "Паспорт" },
  { value: "foreign_passport", label: "Загранпаспорт" },
  { value: "driver_license", label: "Водительские права" },
  { value: "snils", label: "СНИЛС" },
  { value: "inn", label: "ИНН" },
  { value: "insurance", label: "Полис ОМС / страховка" },
  { value: "other", label: "Другой документ" },
] as const;

export function identityKindLabel(kind: string | null | undefined): string {
  return IDENTITY_KINDS.find((k) => k.value === kind)?.label ?? kind ?? "Документ";
}

export const identitySchema = z.object({
  ...common,
  kind: z.string().max(64),
  fullName: z.string().trim().max(512, "Слишком длинное ФИО"),
  number: z.string().trim().max(512, "Слишком длинный номер"),
  issuedBy: z.string().trim().max(512, "Слишком длинное поле"),
  issueDate: z.string().max(32),
  expiryDate: z.string().max(32),
});

export type IdentityFormValues = z.infer<typeof identitySchema>;

export const EMPTY_IDENTITY: IdentityFormValues = {
  title: "",
  kind: "passport",
  fullName: "",
  number: "",
  issuedBy: "",
  issueDate: "",
  expiryDate: "",
  notes: "",
  tags: [],
  fields: [],
  folderId: null,
  favorite: false,
};

const opt = (v: string) => (v.trim() ? v.trim() : null);
/** Secrets keep inner spacing as typed; Rust strips separators where it matters. */
const secret = (v: string) => (v.trim() ? v.trim() : null);

export function cardToPayload(v: CardFormValues): CardPayload {
  return {
    title: v.title.trim(),
    cardholder: opt(v.cardholder),
    number: secret(v.number),
    expiry: opt(v.expiry),
    cvv: secret(v.cvv),
    pin: secret(v.pin),
    notes: opt(v.notes),
    tags: v.tags,
    fields: cleanFields(v.fields),
  };
}

export function cardToForm(p: CardPayload): Omit<CardFormValues, "folderId" | "favorite"> {
  return {
    title: p.title,
    cardholder: p.cardholder ?? "",
    number: p.number ?? "",
    expiry: p.expiry ?? "",
    cvv: p.cvv ?? "",
    pin: p.pin ?? "",
    notes: p.notes ?? "",
    tags: p.tags,
    fields: p.fields ?? [],
  };
}

export function identityToPayload(v: IdentityFormValues): IdentityPayload {
  return {
    title: v.title.trim(),
    kind: opt(v.kind),
    fullName: opt(v.fullName),
    number: secret(v.number),
    issuedBy: opt(v.issuedBy),
    issueDate: opt(v.issueDate),
    expiryDate: opt(v.expiryDate),
    notes: opt(v.notes),
    tags: v.tags,
    fields: cleanFields(v.fields),
  };
}

export function identityToForm(
  p: IdentityPayload,
): Omit<IdentityFormValues, "folderId" | "favorite"> {
  return {
    title: p.title,
    kind: p.kind ?? "",
    fullName: p.fullName ?? "",
    number: p.number ?? "",
    issuedBy: p.issuedBy ?? "",
    issueDate: p.issueDate ?? "",
    expiryDate: p.expiryDate ?? "",
    notes: p.notes ?? "",
    tags: p.tags,
    fields: p.fields ?? [],
  };
}

/** `2031-05-17` → `17.05.2031`; anything else is shown as typed. */
export function formatDocDate(value: string | null): string | null {
  if (!value) return null;
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
  return m ? `${m[3]}.${m[2]}.${m[1]}` : value;
}
