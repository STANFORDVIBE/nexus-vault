import { useCallback, useDeferredValue, useState, type ReactNode } from "react";
import { Navigate, useParams, useSearchParams } from "react-router";
import { MousePointerClickIcon, Trash2Icon } from "lucide-react";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { EmptyState } from "@/components/common/EmptyState";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { ItemDetailPane } from "@/features/items/ItemDetailPane";
import { ItemList } from "@/features/items/ItemList";
import { CardForm } from "@/features/items/CardForm";
import { IdentityForm } from "@/features/items/IdentityForm";
import { EMPTY_CARD, EMPTY_IDENTITY } from "@/features/items/itemSchemas";
import { LoginForm } from "@/features/items/LoginForm";
import { EMPTY_LOGIN } from "@/features/items/loginSchema";
import { NoteEditor } from "@/features/items/NoteEditor";
import { EMPTY_NOTE } from "@/features/items/noteDraft";
import { FOLDER_EMPTY, VIEWS } from "@/features/items/views";
import { useVaultQuery } from "@/hooks/useVaultQuery";
import { ITEMS_FORMS, plural } from "@/lib/format";
import { errorMessage } from "@/services/errors";
import { emptyTrash, listFolders, listItems } from "@/services/vault";
import { invalidate } from "@/store/session";
import type { ItemFilter, ItemSort, ItemType } from "@/types/vault";

type NewKind = "login" | "note" | "card" | "identity";
const NEW_KIND: Record<ItemType, NewKind> = {
  login: "login",
  secure_note: "note",
  card: "card",
  identity: "identity",
};

const SORT_KEY = "nexus-vault:sort";

function loadSort(): ItemSort {
  try {
    const value = localStorage.getItem(SORT_KEY);
    if (value === "updated" || value === "created" || value === "title") return value;
  } catch {
    // default
  }
  return "updated";
}

interface ItemsPageProps {
  view: keyof typeof VIEWS | "folder";
}

export function ItemsPage({ view }: ItemsPageProps) {
  const { folderId } = useParams();
  const [params, setParams] = useSearchParams();
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<ItemSort>(loadSort);
  const [confirmEmpty, setConfirmEmpty] = useState(false);
  const deferredQuery = useDeferredValue(query);

  const { data: folders } = useVaultQuery("folders", listFolders);
  const folder = view === "folder" ? folders?.find((f) => f.id === folderId) : undefined;
  const config = view === "folder" ? undefined : VIEWS[view];

  const filter: ItemFilter = {
    view,
    folderId: view === "folder" ? folderId : undefined,
    query: deferredQuery.trim() || undefined,
    sort,
  };
  const { data: items } = useVaultQuery(`items:${JSON.stringify(filter)}`, () => listItems(filter));

  const selectedId = params.get("item");
  const creating = params.get("new");
  const editing = params.get("edit") === "1";

  const setSelection = useCallback(
    (next: { item?: string | null; new?: NewKind; edit?: boolean }) => {
      const p = new URLSearchParams();
      if (next.item) p.set("item", next.item);
      if (next.new) p.set("new", next.new);
      if (next.edit) p.set("edit", "1");
      setParams(p, { replace: true });
    },
    [setParams],
  );

  const onEditDone = useCallback((id: string | null) => setSelection({ item: id }), [setSelection]);

  function create(type: ItemType) {
    setSelection({ new: NEW_KIND[type] });
  }

  function changeSort(next: ItemSort) {
    setSort(next);
    try {
      localStorage.setItem(SORT_KEY, next);
    } catch {
      // not persisted
    }
  }

  async function doEmptyTrash() {
    try {
      const count = await emptyTrash();
      invalidate();
      setSelection({});
      toast.success(`Удалено: ${count} ${plural(count, ITEMS_FORMS)}`);
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  if (view === "folder" && folders && !folder) {
    // folder was deleted (or a stale link): go to the folder list
    return <Navigate to="/folders" replace />;
  }

  const title = folder?.name ?? config?.title ?? "";
  const trash = view === "trash";

  let detail: ReactNode;
  if (creating === "login") {
    detail = (
      <LoginForm
        key="new-login"
        initial={{ ...EMPTY_LOGIN, folderId: folder?.id ?? null, favorite: view === "favorites" }}
        onSaved={(id) => setSelection({ item: id })}
        onCancel={() => setSelection({})}
      />
    );
  } else if (creating === "card" || creating === "identity") {
    const base = { folderId: folder?.id ?? null, favorite: view === "favorites" };
    const props = {
      onSaved: (id: string) => setSelection({ item: id }),
      onCancel: () => setSelection({}),
    };
    detail =
      creating === "card" ? (
        <CardForm key="new-card" initial={{ ...EMPTY_CARD, ...base }} {...props} />
      ) : (
        <IdentityForm key="new-identity" initial={{ ...EMPTY_IDENTITY, ...base }} {...props} />
      );
  } else if (creating === "note") {
    detail = (
      <NoteEditor
        key="new-note"
        initial={{ ...EMPTY_NOTE, folderId: folder?.id ?? null, favorite: view === "favorites" }}
        onDone={(id) => setSelection({ item: id })}
      />
    );
  } else if (selectedId) {
    detail = (
      <ItemDetailPane
        id={selectedId}
        editing={editing}
        onEdit={() => setSelection({ item: selectedId, edit: true })}
        onEditDone={onEditDone}
        onGone={() => setSelection({})}
      />
    );
  } else {
    detail = (
      <div className="flex h-full items-center justify-center">
        <EmptyState
          icon={MousePointerClickIcon}
          title="Выберите запись"
          description={
            trash
              ? "Записи в корзине можно восстановить или удалить навсегда"
              : "Или создайте новую: Ctrl+N — пароль, Ctrl+Shift+N — заметка"
          }
        />
      </div>
    );
  }

  return (
    <div className="flex h-full">
      <ItemList
        title={title}
        items={items}
        empty={config?.empty ?? FOLDER_EMPTY}
        query={query}
        onQueryChange={setQuery}
        sort={sort}
        onSortChange={changeSort}
        selectedId={selectedId}
        onSelect={(id) => setSelection({ item: id })}
        onCreate={trash ? undefined : create}
        createType={config?.createType}
        headerExtra={
          trash &&
          !!items?.length && (
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label="Очистить корзину"
                  onClick={() => setConfirmEmpty(true)}
                  className="text-destructive hover:text-destructive"
                >
                  <Trash2Icon />
                </Button>
              </TooltipTrigger>
              <TooltipContent>Очистить корзину</TooltipContent>
            </Tooltip>
          )
        }
      />
      <section aria-label="Запись" className="min-w-0 flex-1 bg-surface/40">
        {detail}
      </section>

      <ConfirmDialog
        open={confirmEmpty}
        onOpenChange={setConfirmEmpty}
        title="Очистить корзину?"
        description="Все записи в корзине будут удалены без возможности восстановления."
        confirmLabel="Очистить"
        destructive
        onConfirm={doEmptyTrash}
      />
    </div>
  );
}
