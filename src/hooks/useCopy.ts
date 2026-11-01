import { useCallback } from "react";
import { toast } from "sonner";
import { errorMessage } from "@/services/errors";
import { copyGenerated, copyItemField } from "@/services/vault";
import type { CopyField } from "@/types/vault";

const FIELD_LABELS: Record<CopyField, string> = {
  username: "Логин",
  email: "Email",
  password: "Пароль",
  website: "Адрес сайта",
  content: "Заметка",
};

function notify(label: string, clearAfter: number) {
  toast.success(`${label} скопирован${label === "Заметка" ? "а" : ""}`, {
    description: clearAfter > 0 ? `Буфер обмена очистится через ${clearAfter} с` : undefined,
  });
}

/** Copying happens in Rust: secrets never pass through the WebView clipboard API. */
export function useCopy() {
  const copyField = useCallback(async (id: string, field: CopyField) => {
    try {
      notify(FIELD_LABELS[field], await copyItemField(id, field));
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
