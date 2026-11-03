import { useState, type FormEvent } from "react";
import { FileDownIcon, TriangleAlertIcon } from "lucide-react";
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
import { SegmentedControl } from "@/components/ui/segmented-control";
import { errorMessage } from "@/services/errors";
import { exportVault } from "@/services/vault";
import type { ExportFormat } from "@/types/vault";

const FORMATS = [
  { value: "csv", label: "CSV" },
  { value: "json", label: "JSON" },
] as const;

const FORMAT_HINTS: Record<ExportFormat, string> = {
  csv: "Таблица в формате Bitwarden — её понимают Bitwarden, 1Password, KeePassXC, Proton Pass и браузеры. Карты и документы попадут туда как заметки.",
  json: "Все поля всех записей без потерь — для архива или своих скриптов.",
};

/** Plaintext export behind a warning and the master password. */
export function ExportVault() {
  const [open, setOpen] = useState(false);
  const [format, setFormat] = useState<ExportFormat>("csv");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!password || busy) return;
    setBusy(true);
    setError(null);
    try {
      const fileName = await exportVault(password, format);
      setPassword("");
      if (fileName) {
        setOpen(false);
        toast.success("Экспорт сохранён", {
          description: `${fileName}. Файл не зашифрован — удалите его, когда он станет не нужен.`,
          duration: 10_000,
        });
      }
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      <Button
        variant="outline"
        onClick={() => {
          setPassword("");
          setError(null);
          setOpen(true);
        }}
      >
        <FileDownIcon />
        Экспорт…
      </Button>

      <Dialog open={open} onOpenChange={(next) => !busy && setOpen(next)}>
        <DialogContent>
          <form onSubmit={(e) => void submit(e)} className="grid gap-5">
            <DialogHeader>
              <DialogTitle>Экспорт без шифрования</DialogTitle>
              <DialogDescription>
                Все пароли и заметки будут сохранены в обычный файл, который может прочитать кто
                угодно.
              </DialogDescription>
            </DialogHeader>

            <div className="flex gap-3 rounded-md border border-amber-500/30 bg-amber-500/10 p-3 text-xs leading-relaxed">
              <TriangleAlertIcon className="size-4 shrink-0 text-amber-500" />
              <span>
                Сохраняйте файл только для переноса в другую программу и удалите его сразу после.
                Для резервных копий используйте зашифрованный формат .nexus. Картинки из заметок в
                экспорт не попадают — они есть только в резервных копиях.
              </span>
            </div>

            <div className="grid gap-2">
              <span className="text-sm font-medium">Формат</span>
              <SegmentedControl
                aria-label="Формат экспорта"
                value={format}
                options={FORMATS}
                onValueChange={(v: ExportFormat) => setFormat(v)}
              />
              <p className="text-xs leading-relaxed text-muted-foreground">
                {FORMAT_HINTS[format]}
              </p>
            </div>

            <div className="grid gap-2">
              <Label htmlFor="export-password">Мастер-пароль</Label>
              <PasswordInput
                id="export-password"
                value={password}
                onChange={(e) => {
                  setPassword(e.target.value);
                  setError(null);
                }}
                aria-invalid={error !== null}
                disabled={busy}
                autoFocus
              />
              {error && <p className="text-xs text-destructive">{error}</p>}
            </div>

            <DialogFooter>
              <Button
                type="button"
                variant="outline"
                onClick={() => setOpen(false)}
                disabled={busy}
              >
                Отмена
              </Button>
              <Button type="submit" disabled={!password || busy}>
                {busy ? "Проверяем пароль…" : "Выбрать, куда сохранить"}
              </Button>
            </DialogFooter>
          </form>
        </DialogContent>
      </Dialog>
    </>
  );
}
