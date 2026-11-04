import { z } from "zod";
import { customFieldsSchema } from "@/features/items/itemSchemas";

export const loginSchema = z.object({
  title: z.string().trim().min(1, "Укажите название").max(200, "Не длиннее 200 символов"),
  website: z.string().trim().max(2048, "Слишком длинный адрес"),
  username: z.string().trim().max(512, "Слишком длинный логин"),
  email: z
    .string()
    .trim()
    .max(512, "Слишком длинный email")
    .refine((v) => !v || /^[^\s@]+@[^\s@]+$/.test(v), "Похоже, это не email"),
  password: z.string().max(4096, "Слишком длинный пароль"),
  // checked for real in Rust; here only the obvious mistakes
  totp: z
    .string()
    .trim()
    .max(2048, "Слишком длинный ключ")
    .refine(
      (v) => !v || /^otpauth:\/\/totp\//i.test(v) || /^[A-Za-z2-7\s=-]{16,}$/.test(v),
      "Нужен секретный ключ (буквы A–Z и цифры 2–7) или ссылка otpauth://",
    ),
  notes: z.string().max(100_000, "Слишком длинные заметки"),
  tags: z.array(z.string()).max(32),
  fields: customFieldsSchema,
  folderId: z.string().nullable(),
  favorite: z.boolean(),
});

export type LoginFormValues = z.infer<typeof loginSchema>;

export const EMPTY_LOGIN: LoginFormValues = {
  title: "",
  website: "",
  username: "",
  email: "",
  password: "",
  totp: "",
  notes: "",
  tags: [],
  fields: [],
  folderId: null,
  favorite: false,
};
