import { z } from "zod";

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
  notes: z.string().max(100_000, "Слишком длинные заметки"),
  tags: z.array(z.string()).max(32),
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
  notes: "",
  tags: [],
  folderId: null,
  favorite: false,
};
