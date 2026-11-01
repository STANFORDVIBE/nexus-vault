import { Link, useNavigate } from "react-router";
import {
  ArchiveIcon,
  ClockIcon,
  CreditCardIcon,
  DicesIcon,
  IdCardIcon,
  KeyRoundIcon,
  LayersIcon,
  PlusIcon,
  StarIcon,
  StickyNoteIcon,
  Trash2Icon,
  type LucideIcon,
} from "lucide-react";
import { EmptyState } from "@/components/common/EmptyState";
import { ItemAvatar } from "@/components/common/ItemAvatar";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useVaultQuery } from "@/hooks/useVaultQuery";
import { formatRelative } from "@/lib/format";
import { getDashboard } from "@/services/vault";
import type { ItemSummary } from "@/types/vault";

function Stat({
  to,
  icon: Icon,
  label,
  value,
}: {
  to: string;
  icon: LucideIcon;
  label: string;
  value: number | undefined;
}) {
  return (
    <Link
      to={to}
      className="group flex items-center gap-3 rounded-lg border border-border bg-card px-4 py-3.5 transition-colors outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-ring/50"
    >
      <div className="flex size-9 items-center justify-center rounded-md bg-muted text-muted-foreground group-hover:text-brand">
        <Icon className="size-4" strokeWidth={1.75} />
      </div>
      <div>
        <div className="text-xl leading-none font-semibold tabular-nums">{value ?? "–"}</div>
        <div className="mt-1 text-xs text-muted-foreground">{label}</div>
      </div>
    </Link>
  );
}

function ItemLinks({ items, view }: { items: ItemSummary[]; view: string }) {
  return (
    <div className="flex flex-col">
      {items.map((item) => (
        <Link
          key={item.id}
          to={`/${view}?item=${item.id}`}
          className="flex items-center gap-3 rounded-md px-2 py-2 transition-colors outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-ring/50"
        >
          <ItemAvatar type={item.type} title={item.title} />
          <div className="min-w-0 flex-1">
            <div className="truncate text-sm font-medium">{item.title}</div>
            <div className="truncate text-xs text-muted-foreground">
              {item.subtitle || (item.type === "secure_note" ? "Заметка" : " ")}
            </div>
          </div>
          <span className="shrink-0 text-[11px] text-muted-foreground/80">
            {formatRelative(item.updatedAt)}
          </span>
        </Link>
      ))}
    </div>
  );
}

export function DashboardPage() {
  const navigate = useNavigate();
  const { data } = useVaultQuery("dashboard", getDashboard);

  const actions = (
    <>
      <Button variant="outline" onClick={() => void navigate("/generator")}>
        <DicesIcon />
        Генератор
      </Button>
      <Button variant="outline" onClick={() => void navigate("/notes?new=note")}>
        <StickyNoteIcon />
        Заметка
      </Button>
      <Button onClick={() => void navigate("/passwords?new=login")}>
        <PlusIcon />
        Новый пароль
      </Button>
    </>
  );

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 py-8">
      <PageHeader
        title="Обзор"
        description="Всё хранится только на этом компьютере в зашифрованном виде"
        actions={actions}
      />

      <div className="grid grid-cols-2 gap-3 lg:grid-cols-3">
        <Stat to="/items" icon={LayersIcon} label="Всего записей" value={data?.total} />
        <Stat to="/passwords" icon={KeyRoundIcon} label="Пароли" value={data?.logins} />
        <Stat to="/notes" icon={StickyNoteIcon} label="Заметки" value={data?.notes} />
        <Stat to="/cards" icon={CreditCardIcon} label="Карты" value={data?.cards} />
        <Stat to="/documents" icon={IdCardIcon} label="Документы" value={data?.identities} />
        <Stat to="/favorites" icon={StarIcon} label="Избранное" value={data?.favorites} />
      </div>

      {data && data.total === 0 ? (
        <Card>
          <CardContent>
            <EmptyState
              icon={ArchiveIcon}
              title="Хранилище пока пустое"
              description="Добавьте первый пароль — он будет зашифрован сразу при сохранении"
              action={
                <Button onClick={() => void navigate("/passwords?new=login")}>
                  <PlusIcon />
                  Добавить пароль
                </Button>
              }
            />
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 lg:grid-cols-2">
          <Card className="gap-2">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm">
                <ClockIcon className="size-4 text-muted-foreground" />
                Недавно изменённые
              </CardTitle>
            </CardHeader>
            <CardContent className="px-3">
              {data && <ItemLinks items={data.recent} view="items" />}
            </CardContent>
          </Card>
          <Card className="gap-2">
            <CardHeader>
              <CardTitle className="flex items-center gap-2 text-sm">
                <StarIcon className="size-4 text-muted-foreground" />
                Избранное
              </CardTitle>
            </CardHeader>
            <CardContent className="px-3">
              {data?.favoriteItems.length ? (
                <ItemLinks items={data.favoriteItems} view="favorites" />
              ) : (
                <p className="px-2 py-6 text-center text-xs text-muted-foreground">
                  Отметьте звёздочкой записи, которые нужны чаще всего
                </p>
              )}
            </CardContent>
          </Card>
        </div>
      )}

      {!!data?.trash && (
        <Link
          to="/trash"
          className="flex items-center gap-2 self-start text-xs text-muted-foreground hover:text-foreground"
        >
          <Trash2Icon className="size-3.5" />В корзине: {data.trash}
        </Link>
      )}
    </div>
  );
}
