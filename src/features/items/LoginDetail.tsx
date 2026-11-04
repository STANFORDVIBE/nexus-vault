import { useEffect, useState } from "react";
import { ExternalLinkIcon, EyeIcon, EyeOffIcon } from "lucide-react";
import { toast } from "sonner";
import { FieldRow, IconAction } from "@/features/items/FieldRow";
import { PasswordHistory } from "@/features/items/PasswordHistory";
import { TotpRow } from "@/features/items/TotpRow";
import { CustomFieldsDetail, NotesBlock } from "@/features/items/TypedDetails";
import { useCopy } from "@/hooks/useCopy";
import { errorMessage } from "@/services/errors";
import { openItemWebsite, revealPassword } from "@/services/vault";
import type { LoginView } from "@/types/vault";

/** A revealed password is hidden again automatically after this long. */
const REVEAL_MS = 30_000;

interface LoginDetailProps {
  id: string;
  login: LoginView;
  readOnly: boolean;
}

export function LoginDetail({ id, login, readOnly }: LoginDetailProps) {
  const { copyField } = useCopy();
  const [revealed, setRevealed] = useState<string | null>(null);

  // the parent keys this component by item id, so switching items starts hidden
  useEffect(() => {
    if (revealed === null) return;
    const timer = window.setTimeout(() => setRevealed(null), REVEAL_MS);
    return () => window.clearTimeout(timer);
  }, [revealed]);

  async function toggleReveal() {
    if (revealed !== null) {
      setRevealed(null);
      return;
    }
    try {
      setRevealed(await revealPassword(id));
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  const copy = readOnly
    ? undefined
    : (field: Parameters<typeof copyField>[1]) => () => void copyField(id, field);

  const rows = [
    login.username && (
      <FieldRow key="username" label="Логин" onCopy={copy?.("username")}>
        {login.username}
      </FieldRow>
    ),
    login.email && (
      <FieldRow key="email" label="Email" onCopy={copy?.("email")}>
        {login.email}
      </FieldRow>
    ),
    login.hasPassword && (
      <FieldRow
        key="password"
        label="Пароль"
        mono
        onCopy={copy?.("password")}
        actions={
          !readOnly && (
            <IconAction
              label={revealed !== null ? "Скрыть пароль" : "Показать пароль"}
              icon={revealed !== null ? EyeOffIcon : EyeIcon}
              onClick={() => void toggleReveal()}
              pressed={revealed !== null}
            />
          )
        }
      >
        <span className={revealed === null ? "tracking-[0.2em]" : "select-text"}>
          {revealed ?? "••••••••••••"}
        </span>
      </FieldRow>
    ),
    login.hasTotp && <TotpRow key="totp" id={id} onCopy={copy?.("totp")} />,
    login.website && (
      <FieldRow
        key="website"
        label="Сайт"
        onCopy={copy?.("website")}
        actions={
          !readOnly && (
            <IconAction
              label="Открыть в браузере"
              icon={ExternalLinkIcon}
              onClick={() =>
                void openItemWebsite(id).catch((e: unknown) => toast.error(errorMessage(e)))
              }
            />
          )
        }
      >
        {login.website}
      </FieldRow>
    ),
  ].filter(Boolean);

  return (
    <div className="flex flex-col gap-5">
      {rows.length > 0 ? (
        <div className="divide-y divide-border rounded-lg border border-border bg-card">{rows}</div>
      ) : (
        <p className="text-sm text-muted-foreground">В этой записи нет данных для входа</p>
      )}

      <PasswordHistory id={id} history={login.passwordHistory} readOnly={readOnly} />
      <CustomFieldsDetail id={id} fields={login.fields} readOnly={readOnly} />
      <NotesBlock notes={login.notes} />
    </div>
  );
}
