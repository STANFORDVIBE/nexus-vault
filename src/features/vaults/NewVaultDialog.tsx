import { useState, type FormEvent } from "react";
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
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { errorMessage } from "@/services/errors";
import { addNewVault } from "@/services/vault";
import { useSessionStore } from "@/store/session";

const LOCATIONS = [
  { value: "default", label: "Папка приложения" },
  { value: "custom", label: "Выбрать папку…" },
] as const;

interface NewVaultDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

/** Asks for a name and location, then switches to the new vault's onboarding screen. */
export function NewVaultDialog({ open, onOpenChange }: NewVaultDialogProps) {
  const [name, setName] = useState("");
  const [location, setLocation] = useState<"default" | "custom">("default");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const setStatus = useSessionStore((s) => s.setStatus);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!name.trim() || busy) return;
    setBusy(true);
    setError(null);
    try {
      const status = await addNewVault(name, location === "custom");
      if (!status) return; // folder dialog cancelled
      onOpenChange(false);
      setName("");
      setStatus(status);
      toast("Придумайте мастер-пароль для нового хранилища");
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={(next) => !busy && onOpenChange(next)}>
      <DialogContent>
        <form onSubmit={(e) => void submit(e)} className="grid gap-5">
          <DialogHeader>
            <DialogTitle>Новое хранилище</DialogTitle>
            <DialogDescription>
              Отдельный файл со своим мастер-паролем — например, для работы или семьи. Текущее
              хранилище будет заблокировано, его данные не изменятся.
            </DialogDescription>
          </DialogHeader>

          <div className="grid gap-2">
            <Label htmlFor="vault-name">Название</Label>
            <Input
              id="vault-name"
              value={name}
              onChange={(e) => {
                setName(e.target.value);
                setError(null);
              }}
              maxLength={64}
              placeholder="Например, Работа"
              disabled={busy}
              autoFocus
            />
          </div>

          <div className="grid gap-2">
            <span className="text-sm font-medium">Где хранить файл</span>
            <SegmentedControl
              aria-label="Где хранить файл"
              value={location}
              options={LOCATIONS}
              onValueChange={(v: "default" | "custom") => setLocation(v)}
            />
            <p className="text-xs leading-relaxed text-muted-foreground">
              {location === "default"
                ? "Рядом с основным хранилищем, в папке данных приложения."
                : "Например, на другом диске или в папке, которую синхронизирует облако."}
            </p>
          </div>

          {error && <p className="text-xs text-destructive">{error}</p>}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={busy}
            >
              Отмена
            </Button>
            <Button type="submit" disabled={busy || !name.trim()}>
              {busy ? "Создаём…" : "Продолжить"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
