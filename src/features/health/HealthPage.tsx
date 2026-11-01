import type { ReactNode } from "react";
import { Link } from "react-router";
import {
  CopyIcon,
  HistoryIcon,
  ShieldAlertIcon,
  ShieldCheckIcon,
  ShieldIcon,
  type LucideIcon,
} from "lucide-react";
import { cn } from "cn";
import { EmptyState } from "@/components/common/EmptyState";
import { ItemAvatar } from "@/components/common/ItemAvatar";
import { PageHeader } from "@/components/layout/PageHeader";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useVaultQuery } from "@/hooks/useVaultQuery";
import { plural } from "@/lib/format";
import { getPasswordHealth } from "@/services/vault";
import type { ItemSummary } from "@/types/vault";

const LEVEL_LABELS = ["Очень слабый", "Слабый"];
const PASSWORDS = ["пароль", "пароля", "паролей"] as const;
const DAYS = ["день", "дня", "дней"] as const;

function ages(days: number): string {
  if (days >= 730) {
    const years = Math.floor(days / 365);
    return `${years} ${plural(years, ["год", "года", "лет"])}`;
  }
  if (days >= 365) return "больше года";
  return `${days} ${plural(days, DAYS)}`;
}

function ItemRow({ item, note }: { item: ItemSummary; note?: ReactNode }) {
  return (
    <Link
      to={`/passwords?item=${item.id}&edit=1`}
      title="Открыть и сменить пароль"
      className="flex items-center gap-3 rounded-md px-2 py-2 transition-colors outline-none hover:bg-hover focus-visible:ring-2 focus-visible:ring-ring/50"
    >
      <ItemAvatar type={item.type} title={item.title} />
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium">{item.title}</div>
        {item.subtitle && (
          <div className="truncate text-xs text-muted-foreground">{item.subtitle}</div>
        )}
      </div>
      {note && <span className="shrink-0 text-xs text-muted-foreground">{note}</span>}
    </Link>
  );
}

function Section({
  icon: Icon,
  title,
  hint,
  count,
  children,
}: {
  icon: LucideIcon;
  title: string;
  hint: string;
  count: number;
  children: ReactNode;
}) {
  return (
    <Card className="gap-2">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-sm">
          <Icon className="size-4 text-muted-foreground" />
          {title}
          <span className="ml-auto text-xs font-normal text-muted-foreground tabular-nums">
            {count}
          </span>
        </CardTitle>
        <p className="text-xs text-muted-foreground">{hint}</p>
      </CardHeader>
      <CardContent>
        {count === 0 ? (
          <p className="px-2 py-2 text-sm text-muted-foreground">Всё в порядке</p>
        ) : (
          <div className="flex flex-col">{children}</div>
        )}
      </CardContent>
    </Card>
  );
}

export function HealthPage() {
  const { data } = useVaultQuery("health", getPasswordHealth);

  const header = (
    <PageHeader
      title="Проверка паролей"
      description="Проверка идёт только на этом компьютере: пароли никуда не отправляются"
    />
  );

  if (data && data.checked === 0) {
    return (
      <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 py-8">
        {header}
        <Card>
          <CardContent>
            <EmptyState
              icon={ShieldIcon}
              title="Пока нечего проверять"
              description="Здесь появится оценка, когда в хранилище будут пароли"
            />
          </CardContent>
        </Card>
      </div>
    );
  }

  const score = data ? Math.round((data.healthy / data.checked) * 100) : null;
  const problems = data ? data.checked - data.healthy : 0;
  const reusedCount = data?.reused.reduce((n, g) => n + g.length, 0) ?? 0;

  return (
    <div className="mx-auto flex w-full max-w-5xl flex-col gap-6 px-8 py-8">
      {header}

      <Card>
        <CardContent className="flex flex-row items-center gap-5">
          <div
            className={cn(
              "flex size-14 items-center justify-center rounded-xl",
              problems === 0 ? "bg-success/15 text-success" : "bg-brand/15 text-brand",
            )}
          >
            {problems === 0 ? (
              <ShieldCheckIcon className="size-7" strokeWidth={1.75} />
            ) : (
              <ShieldAlertIcon className="size-7" strokeWidth={1.75} />
            )}
          </div>
          <div className="min-w-0 flex-1">
            <div className="text-2xl font-semibold tabular-nums">
              {score === null ? "–" : `${score}%`}
              <span className="ml-2 text-sm font-normal text-muted-foreground">
                паролей в порядке
              </span>
            </div>
            <p className="mt-1 text-sm text-muted-foreground">
              {data
                ? problems === 0
                  ? `Проверено ${data.checked} ${plural(data.checked, PASSWORDS)} — проблем нет`
                  : `Проверено ${data.checked} ${plural(data.checked, PASSWORDS)}, стоит поменять ${problems}`
                : "Проверяем…"}
            </p>
            {score !== null && (
              <div className="mt-3 h-1.5 overflow-hidden rounded-full bg-muted">
                <div
                  className={cn("h-full rounded-full", problems === 0 ? "bg-success" : "bg-brand")}
                  style={{ width: `${score}%` }}
                />
              </div>
            )}
          </div>
        </CardContent>
      </Card>

      {data && (
        <div className="grid gap-4 lg:grid-cols-2">
          <Section
            icon={ShieldAlertIcon}
            title="Слабые"
            hint="Короткие или простые — такие подбирают за минуты"
            count={data.weak.length}
          >
            {data.weak.map((w) => (
              <ItemRow key={w.item.id} item={w.item} note={LEVEL_LABELS[w.level] ?? "Слабый"} />
            ))}
          </Section>

          <Section
            icon={CopyIcon}
            title="Повторяются"
            hint="Один пароль на нескольких сайтах: утечка с одного откроет остальные"
            count={reusedCount}
          >
            {data.reused.map((group, i) => (
              <div
                key={group[0]?.id ?? i}
                className="border-b border-border pb-1 last:border-0 [&+&]:pt-1"
              >
                {group.map((item) => (
                  <ItemRow key={item.id} item={item} note={`×${group.length}`} />
                ))}
              </div>
            ))}
          </Section>

          <Section
            icon={HistoryIcon}
            title="Давно не менялись"
            hint={`Не менялись дольше ${data.oldAfterDays} ${plural(data.oldAfterDays, DAYS)} — лучше обновить`}
            count={data.old.length}
          >
            {data.old.map((o) => (
              <ItemRow key={o.item.id} item={o.item} note={ages(o.ageDays)} />
            ))}
          </Section>
        </div>
      )}
    </div>
  );
}
