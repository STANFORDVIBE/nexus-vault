import { useEffect, useState, type ReactNode } from "react";
import { useLocation } from "react-router";
import {
  ArchiveIcon,
  DatabaseIcon,
  InfoIcon,
  MonitorIcon,
  MoonIcon,
  PaletteIcon,
  SettingsIcon,
  ShieldIcon,
  SunIcon,
} from "lucide-react";
import { toast } from "sonner";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { SegmentedControl } from "@/components/ui/segmented-control";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { Switch } from "@/components/ui/switch";
import { AutoBackupSettings } from "@/features/backup/AutoBackupSettings";
import { ExportVault } from "@/features/backup/ExportVault";
import { RestoreBackup } from "@/features/backup/RestoreBackup";
import { ChangeMasterPassword } from "@/features/settings/ChangeMasterPassword";
import { HelloSettings } from "@/features/settings/HelloSettings";
import { SettingsRow } from "@/features/settings/SettingsRow";
import { UpdateSettings } from "@/features/settings/UpdateSettings";
import { VaultsSettings } from "@/features/vaults/VaultsSettings";
import { useAppInfo } from "@/hooks/useAppInfo";
import { errorMessage } from "@/services/errors";
import { createBackup } from "@/services/vault";
import { useSettingsStore } from "@/store/settings";
import { useThemeStore, type ThemePreference } from "@/store/theme";
import type { CloseBehavior, Settings } from "@/types/vault";

const AUTO_LOCK = [
  { value: 1, label: "1 минута" },
  { value: 5, label: "5 минут" },
  { value: 15, label: "15 минут" },
  { value: 30, label: "30 минут" },
  { value: 60, label: "1 час" },
  { value: 240, label: "4 часа" },
  { value: 0, label: "Никогда" },
];

const CLIPBOARD = [
  { value: 10, label: "10 секунд" },
  { value: 30, label: "30 секунд" },
  { value: 60, label: "1 минута" },
  { value: 120, label: "2 минуты" },
  { value: 0, label: "Не очищать" },
];

const CLOSE: { value: CloseBehavior; label: string }[] = [
  { value: "lock_to_tray", label: "Заблокировать и свернуть в трей" },
  { value: "minimize_to_tray", label: "Свернуть в трей" },
  { value: "exit", label: "Закрыть приложение" },
];

const THEMES = [
  { value: "dark", label: "Тёмная", icon: MoonIcon },
  { value: "light", label: "Светлая", icon: SunIcon },
  { value: "system", label: "Как в Windows", icon: MonitorIcon },
] as const;

function Section({
  id,
  icon: Icon,
  title,
  children,
}: {
  id: string;
  icon: typeof ShieldIcon;
  title: string;
  children: ReactNode;
}) {
  return (
    <Card id={id} className="gap-0 py-0">
      <CardHeader className="py-4">
        <CardTitle className="flex items-center gap-2 text-sm">
          <Icon className="size-4 text-muted-foreground" />
          {title}
        </CardTitle>
      </CardHeader>
      <Separator />
      <CardContent className="divide-y divide-border px-0">{children}</CardContent>
    </Card>
  );
}

function NumberSelect({
  id,
  value,
  options,
  onChange,
}: {
  id: string;
  value: number;
  options: { value: number; label: string }[];
  onChange: (value: number) => void;
}) {
  // keep a non-preset stored value selectable
  const all = options.some((o) => o.value === value)
    ? options
    : [...options, { value, label: String(value) }];
  return (
    <Select value={String(value)} onValueChange={(v) => onChange(Number(v))}>
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

function BackupButton() {
  const [busy, setBusy] = useState(false);
  async function backup() {
    setBusy(true);
    try {
      const fileName = await createBackup();
      if (fileName) toast.success("Резервная копия сохранена", { description: fileName });
    } catch (e) {
      toast.error(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <Button onClick={() => void backup()} disabled={busy}>
      <ArchiveIcon />
      {busy ? "Сохраняем…" : "Создать копию"}
    </Button>
  );
}

export function SettingsPage() {
  const settings = useSettingsStore((s) => s.settings);
  const update = useSettingsStore((s) => s.update);
  const theme = useThemeStore((s) => s.preference);
  const setTheme = useThemeStore((s) => s.setPreference);
  const appInfo = useAppInfo();
  const { hash } = useLocation();

  useEffect(() => {
    // links like /settings#vaults scroll to their section
    if (hash) document.getElementById(hash.slice(1))?.scrollIntoView({ block: "start" });
  }, [hash]);

  async function save(patch: Partial<Settings>) {
    try {
      const saved = await update(patch);
      if (patch.launchAtStartup !== undefined && saved.launchAtStartup !== patch.launchAtStartup) {
        toast.error("Не удалось изменить автозапуск");
      }
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-8">
      <PageHeader title="Настройки" />

      <Section id="vaults" icon={DatabaseIcon} title="Хранилища">
        <VaultsSettings />
      </Section>

      {settings && (
        <>
          <Section id="general" icon={SettingsIcon} title="Общие">
            <SettingsRow
              label="При закрытии окна"
              description="Что делать по крестику. Из трея приложение открывается щелчком по значку"
              htmlFor="close-behavior"
            >
              <Select
                value={settings.closeBehavior}
                onValueChange={(v) => void save({ closeBehavior: v as CloseBehavior })}
              >
                <SelectTrigger id="close-behavior" className="w-72">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {CLOSE.map((o) => (
                    <SelectItem key={o.value} value={o.value}>
                      {o.label}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </SettingsRow>
            <SettingsRow
              label="Запускать вместе с Windows"
              description="Приложение стартует свёрнутым в трей и заблокированным"
              htmlFor="autostart"
            >
              <Switch
                id="autostart"
                checked={settings.launchAtStartup}
                onCheckedChange={(launchAtStartup) => void save({ launchAtStartup })}
              />
            </SettingsRow>
          </Section>

          <Section id="security" icon={ShieldIcon} title="Безопасность">
            <SettingsRow
              label="Автоблокировка"
              description="Через сколько бездействия хранилище заблокируется"
              htmlFor="auto-lock"
            >
              <NumberSelect
                id="auto-lock"
                value={settings.autoLockMinutes}
                options={AUTO_LOCK}
                onChange={(autoLockMinutes) => void save({ autoLockMinutes })}
              />
            </SettingsRow>
            <SettingsRow
              label="Блокировать вместе с Windows"
              description="При Win+L, выходе из сеанса или блокировке экрана"
              htmlFor="lock-system"
            >
              <Switch
                id="lock-system"
                checked={settings.lockOnSystemLock}
                onCheckedChange={(lockOnSystemLock) => void save({ lockOnSystemLock })}
              />
            </SettingsRow>
            <SettingsRow
              label="Очищать буфер обмена"
              description="Скопированный пароль удаляется из буфера через это время. Он не попадает в журнал буфера Windows (Win+V) и облачную синхронизацию"
              htmlFor="clipboard"
            >
              <NumberSelect
                id="clipboard"
                value={settings.clipboardClearSeconds}
                options={CLIPBOARD}
                onChange={(clipboardClearSeconds) => void save({ clipboardClearSeconds })}
              />
            </SettingsRow>
            <SettingsRow
              label="Мастер-пароль"
              description="Забытый мастер-пароль восстановить нельзя — храните его надёжно"
            >
              <ChangeMasterPassword />
            </SettingsRow>
            <HelloSettings />
          </Section>
        </>
      )}

      <Section id="appearance" icon={PaletteIcon} title="Внешний вид">
        <SettingsRow label="Тема">
          <SegmentedControl
            aria-label="Тема"
            value={theme}
            options={THEMES}
            onValueChange={(value: ThemePreference) => setTheme(value)}
          />
        </SettingsRow>
      </Section>

      <Section id="backup" icon={ArchiveIcon} title="Резервные копии">
        {settings && <AutoBackupSettings settings={settings} save={save} />}
        <SettingsRow
          label="Копия в выбранное место"
          description="Файл .nexus зашифрован текущим мастер-паролем. Храните копию на флешке или в облаке — без пароля её не открыть"
        >
          <BackupButton />
        </SettingsRow>
        <SettingsRow
          label="Восстановить из копии"
          description="Текущее хранилище будет заменено. Предыдущая версия сохранится рядом"
        >
          <RestoreBackup />
        </SettingsRow>
        <SettingsRow
          label="Экспорт"
          description="Незашифрованный файл CSV или JSON — чтобы перенести данные в другой менеджер паролей"
        >
          <ExportVault />
        </SettingsRow>
      </Section>

      <Section id="about" icon={InfoIcon} title="О программе">
        <div className="flex flex-col gap-3 px-6 py-4 text-sm">
          <div className="flex items-baseline justify-between">
            <span className="font-medium">Nexus Vault</span>
            <span className="text-xs text-muted-foreground">
              {appInfo ? `версия ${appInfo.version}` : ""}
            </span>
          </div>
          <p className="text-xs leading-relaxed text-muted-foreground">
            Локальный менеджер паролей и защищённых заметок. Работает без аккаунтов и облака: данные
            хранятся только на этом компьютере, в сеть ходит лишь проверка обновлений. Ключ
            выводится из мастер-пароля алгоритмом Argon2id (256 МиБ памяти), каждая запись шифруется
            XChaCha20-Poly1305.
          </p>
          <p className="text-xs leading-relaxed text-muted-foreground">
            Открытые компоненты: Tauri, React, Radix UI, shadcn/ui, Tailwind CSS, Lucide, RustCrypto
            (argon2, chacha20poly1305), SQLite (rusqlite), словарь BIP-39 для парольных фраз, шрифт
            Inter. Лицензии MIT, Apache-2.0, ISC, OFL и CC0.
          </p>
        </div>
        <UpdateSettings settings={settings} save={save} />
      </Section>
    </div>
  );
}
