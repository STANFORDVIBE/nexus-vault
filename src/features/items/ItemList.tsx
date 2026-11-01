import { useEffect, useRef } from "react";
import {
  ArrowDownUpIcon,
  KeyRoundIcon,
  PlusIcon,
  SearchIcon,
  StarIcon,
  StickyNoteIcon,
  XIcon,
} from "lucide-react";
import { cn } from "cn";
import { EmptyState } from "@/components/common/EmptyState";
import { ItemAvatar } from "@/components/common/ItemAvatar";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { ViewConfig } from "@/features/items/views";
import { formatRelative, ITEMS_FORMS, plural } from "@/lib/format";
import { useUiStore } from "@/store/ui";
import type { ItemSort, ItemSummary, ItemType } from "@/types/vault";

const SORT_LABELS: Record<ItemSort, string> = {
  updated: "По изменению",
  created: "По созданию",
  title: "По названию",
};

interface ItemListProps {
  title: string;
  items: ItemSummary[] | undefined;
  empty: ViewConfig["empty"];
  query: string;
  onQueryChange: (query: string) => void;
  sort: ItemSort;
  onSortChange: (sort: ItemSort) => void;
  selectedId: string | null;
  onSelect: (id: string) => void;
  /** Omitted in the trash, where nothing can be created. */
  onCreate?: (type: ItemType) => void;
  createType?: ItemType;
  headerExtra?: React.ReactNode;
}

export function ItemList({
  title,
  items,
  empty,
  query,
  onQueryChange,
  sort,
  onSortChange,
  selectedId,
  onSelect,
  onCreate,
  createType,
  headerExtra,
}: ItemListProps) {
  const searchRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const focusPending = useUiStore((s) => s.searchFocusPending);
  const clearSearchFocus = useUiStore((s) => s.clearSearchFocus);

  useEffect(() => {
    if (!focusPending) return;
    searchRef.current?.focus();
    searchRef.current?.select();
    clearSearchFocus();
  }, [focusPending, clearSearchFocus]);

  // keep the selected row visible when moving with the keyboard
  useEffect(() => {
    if (!selectedId) return;
    listRef.current
      ?.querySelector(`[data-item-id="${CSS.escape(selectedId)}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [selectedId]);

  function onListKeyDown(event: React.KeyboardEvent) {
    if (!items?.length || (event.key !== "ArrowDown" && event.key !== "ArrowUp")) return;
    event.preventDefault();
    const index = items.findIndex((item) => item.id === selectedId);
    const next =
      event.key === "ArrowDown" ? Math.min(index + 1, items.length - 1) : Math.max(index - 1, 0);
    const target = items[next];
    if (target) onSelect(target.id);
  }

  const count = items?.length ?? 0;

  return (
    <section
      aria-label={title}
      className="flex h-full w-[320px] shrink-0 flex-col border-r border-border bg-background"
    >
      <div className="flex flex-col gap-3 px-4 pt-5 pb-3">
        <div className="flex items-center justify-between gap-2">
          <div className="min-w-0">
            <h1 className="truncate text-lg leading-tight">{title}</h1>
            <p className="text-xs text-muted-foreground">
              {items === undefined ? " " : `${count} ${plural(count, ITEMS_FORMS)}`}
            </p>
          </div>
          <div className="flex items-center gap-1">
            {headerExtra}
            {onCreate &&
              (createType ? (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      size="icon-sm"
                      onClick={() => onCreate(createType)}
                      aria-label="Добавить"
                    >
                      <PlusIcon />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>Добавить · Ctrl+N</TooltipContent>
                </Tooltip>
              ) : (
                <DropdownMenu>
                  <DropdownMenuTrigger asChild>
                    <Button size="icon-sm" aria-label="Добавить">
                      <PlusIcon />
                    </Button>
                  </DropdownMenuTrigger>
                  <DropdownMenuContent align="end" className="w-48">
                    <DropdownMenuItem onSelect={() => onCreate("login")}>
                      <KeyRoundIcon />
                      Пароль
                      <span className="ml-auto text-xs text-muted-foreground">Ctrl+N</span>
                    </DropdownMenuItem>
                    <DropdownMenuItem onSelect={() => onCreate("secure_note")}>
                      <StickyNoteIcon />
                      Заметка
                    </DropdownMenuItem>
                  </DropdownMenuContent>
                </DropdownMenu>
              ))}
          </div>
        </div>

        <div className="flex gap-2">
          <div className="relative flex-1">
            <SearchIcon className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              ref={searchRef}
              type="search"
              value={query}
              onChange={(e) => onQueryChange(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Escape" && query) {
                  e.stopPropagation();
                  onQueryChange("");
                } else if (e.key === "ArrowDown" || e.key === "Enter") {
                  const first = items?.[0];
                  if (first) {
                    e.preventDefault();
                    onSelect(first.id);
                    listRef.current?.focus();
                  }
                }
              }}
              placeholder="Поиск · Ctrl+K"
              aria-label="Поиск"
              autoComplete="off"
              spellCheck={false}
              className="h-8 pr-8 pl-8 [&::-webkit-search-cancel-button]:hidden"
            />
            {query && (
              <button
                type="button"
                onClick={() => onQueryChange("")}
                aria-label="Очистить поиск"
                className="absolute top-1/2 right-2 -translate-y-1/2 rounded-sm text-muted-foreground hover:text-foreground"
              >
                <XIcon className="size-3.5" />
              </button>
            )}
          </div>
          <DropdownMenu>
            <Tooltip>
              <TooltipTrigger asChild>
                <DropdownMenuTrigger asChild>
                  <Button variant="outline" size="icon-sm" aria-label="Сортировка">
                    <ArrowDownUpIcon />
                  </Button>
                </DropdownMenuTrigger>
              </TooltipTrigger>
              <TooltipContent>{SORT_LABELS[sort]}</TooltipContent>
            </Tooltip>
            <DropdownMenuContent align="end" className="w-44">
              <DropdownMenuLabel>Сортировка</DropdownMenuLabel>
              <DropdownMenuRadioGroup
                value={sort}
                onValueChange={(value) => onSortChange(value as ItemSort)}
              >
                {(Object.keys(SORT_LABELS) as ItemSort[]).map((key) => (
                  <DropdownMenuRadioItem key={key} value={key}>
                    {SORT_LABELS[key]}
                  </DropdownMenuRadioItem>
                ))}
              </DropdownMenuRadioGroup>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>

      <div
        ref={listRef}
        role="listbox"
        aria-label="Записи"
        tabIndex={-1}
        onKeyDown={onListKeyDown}
        className="flex-1 overflow-y-auto px-2 pb-3 outline-none"
      >
        {items !== undefined && count === 0 ? (
          query ? (
            <EmptyState
              icon={SearchIcon}
              title="Ничего не найдено"
              description={`По запросу «${query}» нет записей`}
            />
          ) : (
            <EmptyState {...empty} />
          )
        ) : (
          items?.map((item) => (
            <button
              key={item.id}
              type="button"
              role="option"
              aria-selected={item.id === selectedId}
              data-item-id={item.id}
              onClick={() => onSelect(item.id)}
              className={cn(
                "flex w-full items-center gap-3 rounded-md px-2.5 py-2 text-left transition-colors outline-none",
                "hover:bg-hover focus-visible:ring-2 focus-visible:ring-ring/50",
                item.id === selectedId && "bg-muted hover:bg-muted",
              )}
            >
              <ItemAvatar type={item.type} title={item.title} />
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-1.5">
                  <span className="truncate text-sm font-medium">{item.title}</span>
                  {item.favorite && (
                    <StarIcon
                      className="size-3 shrink-0 fill-brand text-brand"
                      aria-label="В избранном"
                    />
                  )}
                </div>
                <div className="truncate text-xs text-muted-foreground">
                  {item.subtitle || (item.type === "secure_note" ? "Заметка" : " ")}
                </div>
              </div>
              <span className="shrink-0 self-start pt-0.5 text-[11px] text-muted-foreground/80">
                {formatRelative(item.deletedAt ?? item.updatedAt)}
              </span>
            </button>
          ))
        )}
      </div>
    </section>
  );
}
