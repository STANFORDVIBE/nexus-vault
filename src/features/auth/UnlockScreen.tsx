import { useState, type FormEvent } from "react";
import { LockKeyholeOpenIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { PasswordInput } from "@/components/ui/password-input";
import { AuthLayout } from "@/features/auth/AuthLayout";
import { RestoreBackup } from "@/features/backup/RestoreBackup";
import { errorMessage, hasCode } from "@/services/errors";
import { unlockVault } from "@/services/vault";
import { useSessionStore } from "@/store/session";

export function UnlockScreen() {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const setStatus = useSessionStore((s) => s.setStatus);
  const refresh = useSessionStore((s) => s.refresh);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!password || busy) return;
    setBusy(true);
    setError(null);
    try {
      const status = await unlockVault(password);
      setPassword("");
      setStatus(status);
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
      // a damaged file is reported as integrity: show the recovery screen
      if (hasCode(e, "integrity")) void refresh();
    }
  }

  return (
    <AuthLayout
      title="Хранилище заблокировано"
      description="Введите мастер-пароль, чтобы продолжить"
      footer={<RestoreBackup variant="ghost" />}
    >
      <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-4">
        <div className="grid gap-2">
          <Label htmlFor="unlock-password" className="sr-only">
            Мастер-пароль
          </Label>
          <PasswordInput
            id="unlock-password"
            placeholder="Мастер-пароль"
            value={password}
            onChange={(e) => {
              setPassword(e.target.value);
              setError(null);
            }}
            aria-invalid={error !== null}
            disabled={busy}
            autoFocus
            className="h-10"
          />
          {error && (
            <p role="alert" className="text-xs text-destructive">
              {error}
            </p>
          )}
        </div>
        <Button type="submit" size="lg" disabled={!password || busy}>
          <LockKeyholeOpenIcon />
          {busy ? "Расшифровываем…" : "Разблокировать"}
        </Button>
      </form>
    </AuthLayout>
  );
}
