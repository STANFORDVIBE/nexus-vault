import { useState, type FormEvent } from "react";
import { FingerprintIcon } from "lucide-react";
import { toast } from "sonner";
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
import { Switch } from "@/components/ui/switch";
import { SettingsRow } from "@/features/settings/SettingsRow";
import { useVaultQuery } from "@/hooks/useVaultQuery";
import { errorMessage, hasCode } from "@/services/errors";
import { disableHello, enableHello, getHelloStatus } from "@/services/vault";
import { invalidate } from "@/store/session";

/** Settings row: turn Windows Hello unlock on (asks the master password) or off. */
export function HelloSettings() {
  const { data: status } = useVaultQuery("hello", getHelloStatus);
  const [open, setOpen] = useState(false);
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!password || busy) return;
    setBusy(true);
    setError(null);
    try {
      await enableHello(password);
      setPassword("");
      setOpen(false);
      invalidate();
      toast.success("Windows Hello включён", {
        description: "Теперь хранилище можно открыть лицом, отпечатком или PIN-кодом",
      });
    } catch (e) {
      // closing the Hello prompt is not an error worth a red message
      if (!hasCode(e, "cancelled")) setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  async function turnOff() {
    try {
      await disableHello();
      invalidate();
      toast.success("Вход через Windows Hello выключен");
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  const unavailable = status !== undefined && !status.available && !status.enabled;

  return (
    <>
      <SettingsRow
        label="Вход через Windows Hello"
        description={
          unavailable
            ? "Windows Hello не настроен на этом компьютере. Включите его в параметрах Windows: Учётные записи → Варианты входа"
            : "Открывать хранилище лицом, отпечатком или PIN-кодом Windows. Мастер-пароль продолжит работать"
        }
        htmlFor="hello"
      >
        <Switch
          id="hello"
          checked={status?.enabled ?? false}
          disabled={!status || unavailable}
          onCheckedChange={(on) => {
            if (on) {
              setError(null);
              setOpen(true);
            } else void turnOff();
          }}
        />
      </SettingsRow>

      <Dialog
        open={open}
        onOpenChange={(next) => {
          if (busy) return;
          setOpen(next);
          setPassword("");
        }}
      >
        <DialogContent className="sm:max-w-md">
          <form onSubmit={(e) => void submit(e)} className="flex flex-col gap-4">
            <DialogHeader>
              <DialogTitle className="flex items-center gap-2">
                <FingerprintIcon className="size-5 text-brand" />
                Включить Windows Hello
              </DialogTitle>
              <DialogDescription>
                Подтвердите мастер-пароль, затем Windows попросит лицо, отпечаток или PIN. Открыть
                хранилище через Hello сможет только тот, кто входит в эту учётную запись Windows на
                этом компьютере.
              </DialogDescription>
            </DialogHeader>
            <div className="grid gap-2">
              <Label htmlFor="hello-password">Мастер-пароль</Label>
              <PasswordInput
                id="hello-password"
                autoFocus
                value={password}
                disabled={busy}
                aria-invalid={error !== null}
                onChange={(e) => {
                  setPassword(e.target.value);
                  setError(null);
                }}
              />
              {error && (
                <p role="alert" className="text-xs text-destructive">
                  {error}
                </p>
              )}
            </div>
            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                disabled={busy}
                onClick={() => setOpen(false)}
              >
                Отмена
              </Button>
              <Button type="submit" disabled={!password || busy}>
                {busy ? "Ждём Windows Hello…" : "Включить"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
