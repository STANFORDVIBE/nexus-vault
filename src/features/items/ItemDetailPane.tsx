import { useEffect, useState } from "react";
import {
  CopyIcon,
  FolderIcon,
  FolderInputIcon,
  MoreHorizontalIcon,
  SquarePenIcon,
  StarIcon,
  Trash2Icon,
  Undo2Icon,
} from "lucide-react";
import { toast } from "sonner";
import { cn } from "cn";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { ItemAvatar } from "@/components/common/ItemAvatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuSub,
  DropdownMenuSubContent,
  DropdownMenuSubTrigger,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import { CardForm } from "@/features/items/CardForm";
import { IdentityForm } from "@/features/items/IdentityForm";
import { cardToForm, identityKindLabel, identityToForm } from "@/features/items/itemSchemas";
import { LoginDetail } from "@/features/items/LoginDetail";
import { LoginForm } from "@/features/items/LoginForm";
import { EMPTY_LOGIN } from "@/features/items/loginSchema";
import { Markdown } from "@/features/items/Markdown";
import { NoteEditor } from "@/features/items/NoteEditor";
import { CardDetail, IdentityDetail } from "@/features/items/TypedDetails";
import { useCopy } from "@/hooks/useCopy";
import { useVaultQuery } from "@/hooks/useVaultQuery";
import { formatDateTime } from "@/lib/format";
import { errorMessage } from "@/services/errors";
import {
  deleteItemPermanently,
  getItem,
  getItemForEdit,
  listFolders,
  moveItemToFolder,
  restoreItem,
  setFavorite,
  trashItem,
} from "@/services/vault";
import { invalidate } from "@/store/session";
import type { ItemContent, ItemDetail, ItemType } from "@/types/vault";

const TYPE_LABELS: Record<ItemType, string> = {
  login: "Пароль",
  secure_note: "Заметка",
  card: "Банковская карта",
  identity: "Документ",
};

const NO_FOLDER = "__none__";

async function run(action: () => Promise<unknown>, success?: string) {
  try {
    await action();
    invalidate();
    if (success) toast.success(success);
    return true;
  } catch (e) {
    toast.error(errorMessage(e));
    return false;
  }
}

interface ItemDetailPaneProps {
  id: string;
  editing: boolean;
  onEdit: () => void;
  onEditDone: (id: string | null) => void;
  /** The item left this view (trashed, restored, deleted). */
  onGone: () => void;
}

export function ItemDetailPane({ id, editing, onEdit, onEditDone, onGone }: ItemDetailPaneProps) {
  const { data: item, error } = useVaultQuery(`item:${id}`, () => getItem(id));

  if (error && !item) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        {errorMessage(error)}
      </div>
    );
  }
  // previous item's data may still be shown for a moment while the new one loads
  if (!item || item.id !== id) return null;

  if (editing && !item.deletedAt) {
    return item.type !== "secure_note" ? (
      <EditLoader key={id} item={item} onDone={onEditDone} />
    ) : (
      <NoteEditor
        key={id}
        itemId={id}
        initial={{
          title: item.payload.title,
          content: item.payload.content,
          tags: item.payload.tags,
          folderId: item.folderId,
          favorite: item.favorite,
        }}
        onDone={onEditDone}
      />
    );
  }

  return <ItemView key={id} item={item} onEdit={onEdit} onGone={onGone} />;
}

/** Fetches the full content (with secrets) before showing the form, so saving drops nothing. */
function EditLoader({ item, onDone }: { item: ItemDetail; onDone: (id: string | null) => void }) {
  const [content, setContent] = useState<ItemContent | null>(null);

  useEffect(() => {
    let cancelled = false;
    getItemForEdit(item.id)
      .then((loaded) => {
        if (!cancelled) setContent(loaded);
      })
      .catch((e: unknown) => {
        toast.error(errorMessage(e));
        onDone(item.id);
      });
    return () => {
      cancelled = true;
    };
    // load once per item: later refetches of `item` must not reset the form
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [item.id]);

  if (!content) return null;
  const meta = { folderId: item.folderId, favorite: item.favorite };
  const props = {
    itemId: item.id,
    initialFavorite: item.favorite,
    onSaved: onDone,
    onCancel: () => onDone(item.id),
  };
  switch (content.type) {
    case "login": {
      const p = content.payload;
      return (
        <LoginForm
          {...props}
          initial={{
            ...EMPTY_LOGIN,
            ...meta,
            title: p.title,
            website: p.website ?? "",
            username: p.username ?? "",
            email: p.email ?? "",
            password: p.password ?? "",
            totp: p.totp ?? "",
            notes: p.notes ?? "",
            tags: p.tags,
            fields: p.fields ?? [],
          }}
        />
      );
    }
    case "card":
      return <CardForm {...props} initial={{ ...cardToForm(content.payload), ...meta }} />;
    case "identity":
      return <IdentityForm {...props} initial={{ ...identityToForm(content.payload), ...meta }} />;
    default:
      return null;
  }
}

function ItemView({
  item,
  onEdit,
  onGone,
}: {
  item: ItemDetail;
  onEdit: () => void;
  onGone: () => void;
}) {
  const [confirmDelete, setConfirmDelete] = useState(false);
  const { data: folders } = useVaultQuery("folders", listFolders);
  const { copyField } = useCopy();
  const trashed = item.deletedAt !== null;
  const folder = folders?.find((f) => f.id === item.folderId);
  const title = item.payload.title;

  return (
    <div className="flex h-full flex-col">
      <div className="flex shrink-0 items-start gap-4 border-b border-border px-6 py-5">
        <ItemAvatar type={item.type} title={title} size="lg" />
        <div className="min-w-0 flex-1">
          <h2 className="truncate text-lg leading-tight">{title}</h2>
          <div className="mt-1.5 flex flex-wrap items-center gap-1.5 text-xs text-muted-foreground">
            <span>
              {item.type === "identity" && item.payload.kind
                ? identityKindLabel(item.payload.kind)
                : TYPE_LABELS[item.type]}
            </span>
            {folder && (
              <>
                <span aria-hidden="true">·</span>
                <span className="inline-flex items-center gap-1">
                  <FolderIcon className="size-3" />
                  {folder.name}
                </span>
              </>
            )}
            {trashed && (
              <Badge variant="destructive" className="ml-1">
                В корзине
              </Badge>
            )}
          </div>
        </div>

        <div className="flex shrink-0 items-center gap-1">
          {trashed ? (
            <>
              <Button
                variant="outline"
                onClick={() =>
                  void run(() => restoreItem(item.id), "Запись восстановлена").then(
                    (ok) => ok && onGone(),
                  )
                }
              >
                <Undo2Icon />
                Восстановить
              </Button>
              <Button variant="destructive" onClick={() => setConfirmDelete(true)}>
                <Trash2Icon />
                Удалить навсегда
              </Button>
            </>
          ) : (
            <>
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    aria-label={item.favorite ? "Убрать из избранного" : "В избранное"}
                    aria-pressed={item.favorite}
                    onClick={() => void run(() => setFavorite(item.id, !item.favorite))}
                  >
                    <StarIcon className={cn(item.favorite && "fill-brand text-brand")} />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>
                  {item.favorite ? "Убрать из избранного" : "В избранное"}
                </TooltipContent>
              </Tooltip>
              <Button variant="outline" onClick={onEdit}>
                <SquarePenIcon />
                Изменить
              </Button>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="icon" aria-label="Ещё">
                    <MoreHorizontalIcon />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-52">
                  {item.type === "secure_note" && (
                    <DropdownMenuItem onSelect={() => void copyField(item.id, "content")}>
                      <CopyIcon />
                      Копировать текст
                    </DropdownMenuItem>
                  )}
                  <DropdownMenuSub>
                    <DropdownMenuSubTrigger>
                      <FolderInputIcon />
                      Переместить в папку
                    </DropdownMenuSubTrigger>
                    <DropdownMenuSubContent className="w-48">
                      <DropdownMenuRadioGroup
                        value={item.folderId ?? NO_FOLDER}
                        onValueChange={(value) =>
                          void run(
                            () => moveItemToFolder(item.id, value === NO_FOLDER ? null : value),
                            "Запись перемещена",
                          )
                        }
                      >
                        <DropdownMenuRadioItem value={NO_FOLDER}>Без папки</DropdownMenuRadioItem>
                        {folders?.map((f) => (
                          <DropdownMenuRadioItem key={f.id} value={f.id}>
                            {f.name}
                          </DropdownMenuRadioItem>
                        ))}
                      </DropdownMenuRadioGroup>
                    </DropdownMenuSubContent>
                  </DropdownMenuSub>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    variant="destructive"
                    onSelect={() =>
                      void run(() => trashItem(item.id), "Запись перемещена в корзину").then(
                        (ok) => ok && onGone(),
                      )
                    }
                  >
                    <Trash2Icon />В корзину
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </>
          )}
        </div>
      </div>

      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto flex max-w-2xl flex-col gap-6 px-6 py-6">
          {item.type === "login" ? (
            <LoginDetail id={item.id} login={item.payload} readOnly={trashed} />
          ) : item.type === "card" ? (
            <CardDetail id={item.id} payload={item.payload} readOnly={trashed} />
          ) : item.type === "identity" ? (
            <IdentityDetail id={item.id} payload={item.payload} readOnly={trashed} />
          ) : item.payload.content.trim() ? (
            <div className="rounded-lg border border-border bg-card px-5 py-4">
              <Markdown>{item.payload.content}</Markdown>
            </div>
          ) : (
            <p className="text-sm text-muted-foreground">Пустая заметка</p>
          )}

          {item.payload.tags.length > 0 && (
            <div className="flex flex-wrap gap-1.5">
              {item.payload.tags.map((tag) => (
                <Badge key={tag} variant="secondary">
                  {tag}
                </Badge>
              ))}
            </div>
          )}

          <dl className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1.5 text-xs text-muted-foreground">
            <dt>Создано</dt>
            <dd>{formatDateTime(item.createdAt)}</dd>
            <dt>Изменено</dt>
            <dd>{formatDateTime(item.updatedAt)}</dd>
            {item.deletedAt && (
              <>
                <dt>Удалено</dt>
                <dd>{formatDateTime(item.deletedAt)}</dd>
              </>
            )}
          </dl>
        </div>
      </div>

      <ConfirmDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title="Удалить навсегда?"
        description={`Запись «${title}» будет удалена без возможности восстановления.`}
        confirmLabel="Удалить"
        destructive
        onConfirm={async () => {
          if (await run(() => deleteItemPermanently(item.id), "Запись удалена")) onGone();
        }}
      />
    </div>
  );
}
