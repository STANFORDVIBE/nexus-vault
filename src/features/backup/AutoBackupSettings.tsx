import { useCallback, useEffect, useState } from "react";
import { FolderOpenIcon, HistoryIcon } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { SettingsRow } from "@/features/settings/SettingsRow";
import { formatRelative } from "@/lib/format";
import { errorMessage } from "@/services/errors";
import {
  chooseAutoBackupFolder,
  getAutoBackupStatus,
  openAutoBackupFolder,
  runAutoBackupNow,
} from "@/services/vault";
import type { AutoBackupStatus, Settings } from "@/types/vault";

const INTERVALS = [
  { value: 6, label: "Каждые 6 часов" },
  { value: 24, label: "Раз в день" },
  { value: 24 * 7, label: "Раз в неделю" },
];

const KEEP = [
  { value: 5, label: "5 последних" },
  { value: 10, label: "10 последних" },
  { value: 30, label: "30 последних" },
];

function ChoiceSelect({
  id,
  value,
  options,
  onChange,
  disabled,
}: {
  id: string;
  value: number;
  options: { value: number; label: string }[];
  onChange: (value: number) => void;
  disabled?: boolean;
}) {
  const all = options.some((o) => o.value === value)
    ? options
    : [...options, { value, label: String(value) }];
  return (
    <Select value={String(value)} onValueChange={(v) => onChange(Number(v))} disabled={disabled}>
      <SelectTrigger id={id} className="w-44">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {all.map((o) => (
          <SelectItem key={o.value} value={String(o.value)}>
            {o.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}

interface AutoBackupSettingsProps {
  settings: Settings;
  save: (patch: Partial<Settings>) => Promise<void>;
}

/** Rows of the "Backups" settings section that control automatic copies. */
export function AutoBackupSettings({ settings, save }: AutoBackupSettingsProps) {
  const [status, setStatus] = useState<AutoBackupStatus | null>(null);
  const [busy, setBusy] = useState(false);

  const reload = useCallback(() => {
    getAutoBackupStatus()
      .then(setStatus)
      .catch(() => setStatus(null));
  }, []);

  // the folder depends on settings, so refresh whenever they change
  useEffect(reload, [reload, settings.autoBackupDir]);

  async function backupNow() {
    setBusy(true);
    try {
      const fileName = await runAutoBackupNow();
      toast.success("Копия сохранена", { description: fileName });
      reload();
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  async function changeFolder() {
    try {
      const folder = await chooseAutoBackupFolder();
      if (folder) await save({ autoBackupDir: folder });
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  const off = !settings.autoBackup;

  return (
    <>
      <SettingsRow
        label="Автоматические копии"
        description={
          status?.lastAt
            ? `Последняя — ${formatRelative(status.lastAt)}. Сейчас хранится: ${status.count}. Если ничего не менялось, новая копия не делается`
            : "Зашифрованная копия делается сама, пока хранилище открыто и в нём что-то менялось"
        }
        htmlFor="auto-backup"
      >
        <Switch
          id="auto-backup"
          checked={settings.autoBackup}
          onCheckedChange={(autoBackup) => void save({ autoBackup })}
        />
      </SettingsRow>
      <SettingsRow label="Как часто" htmlFor="auto-backup-interval">
        <ChoiceSelect
          id="auto-backup-interval"
          value={settings.autoBackupIntervalHours}
          options={INTERVALS}
          onChange={(autoBackupIntervalHours) => void save({ autoBackupIntervalHours })}
          disabled={off}
        />
      </SettingsRow>
      <SettingsRow
        label="Сколько хранить"
        description="Более старые автокопии удаляются. Копии, сделанные вручную, не трогаются"
        htmlFor="auto-backup-keep"
      >
        <ChoiceSelect
          id="auto-backup-keep"
          value={settings.autoBackupKeep}
          options={KEEP}
          onChange={(autoBackupKeep) => void save({ autoBackupKeep })}
          disabled={off}
        />
      </SettingsRow>
      <div className="flex flex-col gap-3 px-6 py-4">
        <div className="flex min-w-0 flex-col gap-0.5">
          <span className="text-sm font-medium">Папка для автокопий</span>
          <span
            className="truncate font-mono text-[11px] text-muted-foreground"
            title={status?.folder}
          >
            {status?.folder ?? " "}
          </span>
          <span className="text-xs leading-relaxed text-muted-foreground">
            Надёжнее всего — другой диск или папка, которую синхронизирует облако (OneDrive, Яндекс
            Диск): тогда копии переживут поломку компьютера.
          </span>
        </div>
        <div className="flex flex-wrap gap-2">
          <Button variant="outline" size="sm" onClick={() => void changeFolder()}>
            Изменить…
          </Button>
          {settings.autoBackupDir && (
            <Button variant="ghost" size="sm" onClick={() => void save({ autoBackupDir: null })}>
              По умолчанию
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            onClick={() =>
              void openAutoBackupFolder().catch((e: unknown) => toast.error(errorMessage(e)))
            }
          >
            <FolderOpenIcon />
            Открыть папку
          </Button>
          <Button variant="ghost" size="sm" onClick={() => void backupNow()} disabled={busy}>
            <HistoryIcon />
            {busy ? "Сохраняем…" : "Сделать копию сейчас"}
          </Button>
        </div>
      </div>
    </>
  );
}
