import { useState, type FormEvent } from "react";
import { TriangleAlertIcon } from "lucide-react";
import { StrengthMeter } from "@/components/common/StrengthMeter";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";
import { PasswordInput } from "@/components/ui/password-input";
import { AuthLayout } from "@/features/auth/AuthLayout";
import { RestoreBackup } from "@/features/backup/RestoreBackup";
import { estimateStrength } from "@/lib/password-strength";
import { errorMessage } from "@/services/errors";
import { createVault } from "@/services/vault";
import { useSessionStore } from "@/store/session";

export const MIN_MASTER_LENGTH = 10;

export function OnboardingScreen() {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [understood, setUnderstood] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const setStatus = useSessionStore((s) => s.setStatus);

  const strength = estimateStrength(password);
  const tooShort = [...password].length < MIN_MASTER_LENGTH;
  const mismatch = confirm.length > 0 && confirm !== password;
  const canSubmit = !tooShort && confirm === password && understood && !busy;

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      const status = await createVault(password);
      setPassword("");
      setConfirm("");
      setStatus(status);
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  }

  return (
    <AuthLayout
      title="Создайте хранилище"
      description="Все данные шифруются на этом компьютере мастер-паролем. Он нигде не хранится и не отправляется в сеть."
      footer={
        <>
          <span className="text-xs text-muted-foreground">Уже есть резервная копия?</span>
          <RestoreBackup variant="ghost" />
        </>
      }
    >
      <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-5">
        <div className="grid gap-2">
          <Label htmlFor="master">Мастер-пароль</Label>
          <PasswordInput
            id="master"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            disabled={busy}
            autoFocus
          />
          <StrengthMeter
            level={password ? strength.level : 0}
            label={
              password
                ? strength.label
                : `Не короче ${MIN_MASTER_LENGTH} символов. Лучше — фраза из нескольких слов`
            }
            hint={
              password && tooShort ? `ещё ${MIN_MASTER_LENGTH - [...password].length}` : undefined
            }
          />
        </div>

        <div className="grid gap-2">
          <Label htmlFor="master-confirm">Повторите пароль</Label>
          <PasswordInput
            id="master-confirm"
            value={confirm}
            onChange={(e) => setConfirm(e.target.value)}
            aria-invalid={mismatch}
            disabled={busy}
          />
          {mismatch && <p className="text-xs text-destructive">Пароли не совпадают</p>}
        </div>

        <div className="flex gap-3 rounded-md border border-amber-500/30 bg-amber-500/10 p-3 text-xs leading-relaxed">
          <TriangleAlertIcon className="size-4 shrink-0 text-amber-500" />
          <span>
            Восстановить забытый мастер-пароль невозможно — ни нам, ни кому-либо ещё. Без него
            данные не расшифровать.
          </span>
        </div>

        <label className="flex cursor-pointer items-start gap-2.5 text-sm">
          <Checkbox
            checked={understood}
            onCheckedChange={(v) => setUnderstood(v === true)}
            disabled={busy}
            className="mt-0.5"
          />
          <span>Я понимаю, что пароль нельзя восстановить</span>
        </label>

        {error && <p className="text-sm text-destructive">{error}</p>}

        <Button type="submit" size="lg" disabled={!canSubmit}>
          {busy ? "Создаём хранилище…" : "Создать хранилище"}
        </Button>
      </form>
    </AuthLayout>
  );
}
