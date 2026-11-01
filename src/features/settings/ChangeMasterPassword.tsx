import { useState, type FormEvent } from "react";
import { KeyRoundIcon } from "lucide-react";
import { toast } from "sonner";
import { StrengthMeter } from "@/components/common/StrengthMeter";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Label } from "@/components/ui/label";
import { PasswordInput } from "@/components/ui/password-input";
import { MIN_MASTER_LENGTH } from "@/features/auth/OnboardingScreen";
import { estimateStrength } from "@/lib/password-strength";
import { errorMessage, hasCode } from "@/services/errors";
import { changeMasterPassword } from "@/services/vault";

export function ChangeMasterPassword() {
  const [open, setOpen] = useState(false);
  const [current, setCurrent] = useState("");
  const [next, setNext] = useState("");
  const [confirm, setConfirm] = useState("");
  const [error, setError] = useState<{ field: "current" | "next"; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  const strength = estimateStrength(next);
  const tooShort = [...next].length < MIN_MASTER_LENGTH;
  const mismatch = confirm.length > 0 && confirm !== next;
  const canSubmit = current && !tooShort && confirm === next && !busy;

  function reset(nextOpen: boolean) {
    if (busy) return;
    setOpen(nextOpen);
    setCurrent("");
    setNext("");
    setConfirm("");
    setError(null);
  }

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!canSubmit) return;
    setBusy(true);
    setError(null);
    try {
      await changeMasterPassword(current, next);
      setBusy(false);
      reset(false);
      toast.success("Мастер-пароль изменён", {
        description: "Старые резервные копии открываются старым паролем",
      });
    } catch (e) {
      setError({
        field: hasCode(e, "invalid_password") ? "current" : "next",
        text: errorMessage(e),
      });
      setBusy(false);
    }
  }

  return (
    <>
      <Button variant="outline" onClick={() => setOpen(true)}>
        <KeyRoundIcon />
        Сменить мастер-пароль
      </Button>
      <Dialog open={open} onOpenChange={reset}>
        <DialogContent>
          <form onSubmit={(e) => void submit(e)} className="grid gap-5">
            <DialogHeader>
              <DialogTitle>Смена мастер-пароля</DialogTitle>
              <DialogDescription>
                Данные не перешифровываются целиком: меняется только ключ, которым защищено
                хранилище, поэтому это быстро и безопасно.
              </DialogDescription>
            </DialogHeader>
            <div className="grid gap-2">
              <Label htmlFor="cmp-current">Текущий пароль</Label>
              <PasswordInput
                id="cmp-current"
                value={current}
                onChange={(e) => setCurrent(e.target.value)}
                aria-invalid={error?.field === "current"}
                disabled={busy}
                autoFocus
              />
              {error?.field === "current" && (
                <p className="text-xs text-destructive">{error.text}</p>
              )}
            </div>
            <div className="grid gap-2">
              <Label htmlFor="cmp-new">Новый пароль</Label>
              <PasswordInput
                id="cmp-new"
                value={next}
                onChange={(e) => setNext(e.target.value)}
                disabled={busy}
              />
              <StrengthMeter
                level={next ? strength.level : 0}
                label={next ? strength.label : `Не короче ${MIN_MASTER_LENGTH} символов`}
              />
            </div>
            <div className="grid gap-2">
              <Label htmlFor="cmp-confirm">Повторите новый пароль</Label>
              <PasswordInput
                id="cmp-confirm"
                value={confirm}
                onChange={(e) => setConfirm(e.target.value)}
                aria-invalid={mismatch}
                disabled={busy}
              />
              {mismatch && <p className="text-xs text-destructive">Пароли не совпадают</p>}
              {error?.field === "next" && <p className="text-xs text-destructive">{error.text}</p>}
            </div>
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => reset(false)} disabled={busy}>
                Отмена
              </Button>
              <Button type="submit" disabled={!canSubmit}>
                {busy ? "Меняем…" : "Сменить пароль"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
