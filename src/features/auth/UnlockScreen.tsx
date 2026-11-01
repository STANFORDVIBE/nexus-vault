import { useEffect, useState, type FormEvent } from "react";
import { FingerprintIcon, LockKeyholeOpenIcon } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { PasswordInput } from "@/components/ui/password-input";
import { AuthLayout } from "@/features/auth/AuthLayout";
import { RestoreBackup } from "@/features/backup/RestoreBackup";
import { ForgotPasswordDialog } from "@/features/vaults/ForgotPasswordDialog";
import { VaultSwitcher } from "@/features/vaults/VaultSwitcher";
import { errorMessage, hasCode } from "@/services/errors";
import { getHelloStatus, unlockVault, unlockWithHello } from "@/services/vault";
import { useSessionStore } from "@/store/session";

export function UnlockScreen() {
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const setStatus = useSessionStore((s) => s.setStatus);
  const refresh = useSessionStore((s) => s.refresh);
  const vaultName = useSessionStore((s) => s.status?.vault.name);
  const vaultId = useSessionStore((s) => s.status?.vault.id);
  const [hello, setHello] = useState(false);

  useEffect(() => {
    let cancelled = false;
    getHelloStatus()
      .then((s) => !cancelled && setHello(s.available && s.enabled))
      .catch(() => !cancelled && setHello(false));
    return () => {
      cancelled = true;
    };
  }, [vaultId]);

  async function helloUnlock() {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      setStatus(await unlockWithHello());
    } catch (e) {
      if (!hasCode(e, "cancelled")) setError(errorMessage(e));
      setBusy(false);
      if (hasCode(e, "integrity")) void refresh();
    }
  }

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
      description={
        <>
          Введите мастер-пароль хранилища{" "}
          <span className="font-medium text-foreground">«{vaultName}»</span>
        </>
      }
      footer={
        <>
          <ForgotPasswordDialog />
          <div className="flex flex-wrap justify-center gap-1">
            <RestoreBackup variant="ghost" />
            <VaultSwitcher variant="auth" />
          </div>
        </>
      }
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
        {hello && (
          <Button
            type="button"
            variant="outline"
            size="lg"
            disabled={busy}
            onClick={() => void helloUnlock()}
          >
            <FingerprintIcon />
            Windows Hello
          </Button>
        )}
      </form>
    </AuthLayout>
  );
}
