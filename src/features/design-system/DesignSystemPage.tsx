import type { ReactNode } from "react";
import {
  CopyIcon,
  KeyRoundIcon,
  MonitorIcon,
  MoonIcon,
  PlusIcon,
  SearchIcon,
  StarIcon,
  StickyNoteIcon,
  SunIcon,
  Trash2Icon,
} from "lucide-react";
import { toast } from "sonner";
import { PageHeader } from "@/components/layout/PageHeader";
import { EmptyState } from "@/components/common/EmptyState";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { PasswordInput } from "@/components/ui/password-input";
import { SegmentedControl, type SegmentedOption } from "@/components/ui/segmented-control";
import { Switch } from "@/components/ui/switch";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { useThemeStore, type ThemePreference } from "@/store/theme";

const THEME_OPTIONS: readonly SegmentedOption<ThemePreference>[] = [
  { value: "dark", label: "Тёмная", icon: MoonIcon },
  { value: "light", label: "Светлая", icon: SunIcon },
  { value: "system", label: "Системная", icon: MonitorIcon },
];

const SWATCHES = [
  ["background", "Фон"],
  ["sidebar", "Sidebar"],
  ["surface", "Surface"],
  ["elevated", "Elevated"],
  ["hover", "Hover"],
  ["border", "Border"],
  ["foreground", "Текст"],
  ["muted-foreground", "Вторичный текст"],
  ["brand", "Акцент"],
  ["success", "Success"],
  ["destructive", "Danger"],
] as const;

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
        {title}
      </h2>
      {children}
    </section>
  );
}

/** Phase 2 showcase of the design system. Not part of the final product navigation. */
export function DesignSystemPage() {
  const { preference, setPreference } = useThemeStore();

  return (
    <div className="mx-auto flex max-w-4xl flex-col gap-10 px-8 py-8">
      <PageHeader
        title="Дизайн-система"
        description="Витрина компонентов Premium Graphite. Данные на этой странице демонстрационные."
        actions={
          <SegmentedControl
            aria-label="Тема оформления"
            value={preference}
            options={THEME_OPTIONS}
            onValueChange={setPreference}
          />
        }
      />

      <Section title="Цвета">
        <div className="grid grid-cols-[repeat(auto-fill,minmax(120px,1fr))] gap-3">
          {SWATCHES.map(([token, label]) => (
            <div key={token} className="flex flex-col gap-1.5">
              <div
                className="h-12 rounded-md ring-1 ring-border"
                style={{ backgroundColor: `var(--${token})` }}
              />
              <span className="text-xs font-medium">{label}</span>
              <span className="text-[11px] text-muted-foreground">--{token}</span>
            </div>
          ))}
        </div>
      </Section>

      <Section title="Типографика">
        <Card size="sm">
          <CardContent className="gap-2">
            <p className="text-[24px] leading-tight font-semibold tracking-tight">
              Заголовок страницы · 24 px
            </p>
            <p className="text-base font-semibold">Заголовок блока · 16 px</p>
            <p className="text-sm">Основной текст · 14 px. Your digital life. One secure place.</p>
            <p className="text-[13px] text-muted-foreground">Вторичный текст · 13 px</p>
            <p className="text-xs text-muted-foreground">Подпись · 12 px</p>
            <p className="font-mono text-sm">Моноширинный · K7#mq2!vLp9@xR</p>
          </CardContent>
        </Card>
      </Section>

      <Section title="Кнопки">
        <div className="flex flex-wrap items-center gap-2">
          <Button>
            <PlusIcon /> Добавить
          </Button>
          <Button variant="secondary">Вторичная</Button>
          <Button variant="outline">Контурная</Button>
          <Button variant="ghost">Призрачная</Button>
          <Button variant="destructive">
            <Trash2Icon /> Удалить
          </Button>
          <Button variant="link">Ссылка</Button>
          <Button disabled>Недоступна</Button>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <Button size="sm">Маленькая</Button>
          <Button size="lg">Большая</Button>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="outline" size="icon" aria-label="Копировать">
                <CopyIcon />
              </Button>
            </TooltipTrigger>
            <TooltipContent>Копировать</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button variant="ghost" size="icon-sm" aria-label="В избранное">
                <StarIcon />
              </Button>
            </TooltipTrigger>
            <TooltipContent>В избранное</TooltipContent>
          </Tooltip>
        </div>
      </Section>

      <Section title="Поля ввода">
        <Card size="sm">
          <CardContent className="grid grid-cols-2 gap-x-6 gap-y-4">
            <div className="flex flex-col gap-2">
              <Label htmlFor="ds-title">Название</Label>
              <Input id="ds-title" placeholder="Например, GitHub" />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="ds-password">Пароль</Label>
              <PasswordInput id="ds-password" defaultValue="K7#mq2!vLp9@xR" />
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="ds-search">Поиск</Label>
              <div className="relative">
                <SearchIcon className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
                <Input id="ds-search" placeholder="Поиск…" className="pl-8" />
              </div>
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="ds-invalid">С ошибкой</Label>
              <Input id="ds-invalid" aria-invalid defaultValue="не-адрес" />
              <span className="text-xs text-destructive">Введите корректный адрес сайта</span>
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="ds-disabled">Недоступно</Label>
              <Input id="ds-disabled" disabled placeholder="Только чтение" />
            </div>
            <div className="flex items-center justify-between gap-4 self-end rounded-md border border-border px-3 py-2">
              <Label htmlFor="ds-switch">Блокировать при блокировке Windows</Label>
              <Switch id="ds-switch" defaultChecked />
            </div>
          </CardContent>
        </Card>
      </Section>

      <Section title="Карточки и метки">
        <div className="grid grid-cols-2 gap-3">
          <Card size="sm">
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <KeyRoundIcon className="size-4 text-muted-foreground" /> Всего паролей
              </CardTitle>
              <CardDescription>Демонстрационная карточка статистики</CardDescription>
            </CardHeader>
            <CardContent>
              <span className="text-2xl font-semibold tracking-tight">128</span>
            </CardContent>
          </Card>
          <Card size="sm">
            <CardHeader>
              <CardTitle>Метки типов</CardTitle>
              <CardDescription>Тип записи и статусы</CardDescription>
            </CardHeader>
            <CardContent className="flex-row flex-wrap gap-2">
              <Badge variant="secondary">
                <KeyRoundIcon /> Логин
              </Badge>
              <Badge variant="secondary">
                <StickyNoteIcon /> Заметка
              </Badge>
              <Badge variant="outline">work</Badge>
              <Badge>Избранное</Badge>
              <Badge variant="destructive">В корзине</Badge>
            </CardContent>
            <CardFooter className="text-xs text-muted-foreground">Изменено сегодня</CardFooter>
          </Card>
        </div>
      </Section>

      <Section title="Диалог и уведомления">
        <div className="flex flex-wrap items-center gap-2">
          <Dialog>
            <DialogTrigger asChild>
              <Button variant="outline">Открыть диалог</Button>
            </DialogTrigger>
            <DialogContent>
              <DialogHeader>
                <DialogTitle>Переместить в корзину?</DialogTitle>
                <DialogDescription>
                  Запись можно будет восстановить из корзины. Это демонстрационный диалог, ничего не
                  удаляется.
                </DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <DialogClose asChild>
                  <Button variant="outline">Отмена</Button>
                </DialogClose>
                <DialogClose asChild>
                  <Button variant="destructive">В корзину</Button>
                </DialogClose>
              </DialogFooter>
            </DialogContent>
          </Dialog>
          <Button variant="outline" onClick={() => toast.success("Пароль скопирован")}>
            Уведомление: успех
          </Button>
          <Button
            variant="outline"
            onClick={() =>
              toast.warning("Буфер обмена очистится через 30 с", {
                description: "Другие приложения могут читать буфер обмена.",
              })
            }
          >
            Уведомление: предупреждение
          </Button>
          <Button variant="outline" onClick={() => toast.error("Не удалось сохранить запись")}>
            Уведомление: ошибка
          </Button>
        </div>
      </Section>

      <Section title="Пустое состояние">
        <Card size="sm">
          <EmptyState
            icon={StarIcon}
            title="Пока ничего в избранном"
            description="Отмечайте звёздочкой записи, которыми пользуетесь чаще всего, — они появятся здесь."
            action={<Button size="sm">Перейти ко всем записям</Button>}
          />
        </Card>
      </Section>
    </div>
  );
}
