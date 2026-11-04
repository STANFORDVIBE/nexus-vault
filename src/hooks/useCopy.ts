import { useCallback } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/services/errors";
import { copyGenerated, copyItemField } from "@/services/vault";
import type { CopyField } from "@/types/vault";

type NamedField = Exclude<CopyField, `custom:${number}` | `history:${number}`>;

const FIELD_LABELS: Record<NamedField, string> = {
  username: "Логин",
  email: "Email",
  password: "Пароль",
  website: "Адрес сайта",
  content: "Заметка",
  card_number: "Номер карты",
  card_cvv: "CVV",
  card_pin: "PIN",
  cardholder: "Держатель",
  doc_number: "Номер документа",
  totp: "Код 2FA",
};

function notify(label: string, clearAfter: number) {
  toast.success(`Скопировано: ${label}`, {
    description: clearAfter > 0 ? `Буфер обмена очистится через ${clearAfter} с` : undefined,
  });
}

/** Copying happens in Rust: secrets never pass through the WebView clipboard API. */
export function useCopy() {
  const copyField = useCallback(async (id: string, field: CopyField, label?: string) => {
    try {
      const name =
        label ??
        (field.startsWith("custom:")
          ? "поле"
          : field.startsWith("history:")
            ? "старый пароль"
            : FIELD_LABELS[field as NamedField]);
      notify(name, await copyItemField(id, field));
    } catch (error) {
      toast.error(errorMessage(error));
    }
  }, []);

  const copyValue = useCallback(async (value: string) => {
    try {
      notify("Пароль", await copyGenerated(value));
    } catch (error) {
      toast.error(errorMessage(error));
    }
  }, []);

  return { copyField, copyValue };
}
