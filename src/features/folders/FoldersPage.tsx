import { useState, type FormEvent } from "react";
import { Link } from "react-router";
import {
  FolderIcon,
  FolderPlusIcon,
  MoreHorizontalIcon,
  PencilIcon,
  Trash2Icon,
} from "lucide-react";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { EmptyState } from "@/components/common/EmptyState";
import { PageHeader } from "@/components/layout/PageHeader";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useVaultQuery } from "@/hooks/useVaultQuery";
import { ITEMS_FORMS, plural } from "@/lib/format";
import { errorMessage } from "@/services/errors";
import { createFolder, deleteFolder, listFolders, renameFolder } from "@/services/vault";
import { invalidate } from "@/store/session";
import type { FolderView } from "@/types/vault";

type Editing = { mode: "create" } | { mode: "rename"; folder: FolderView };

function FolderDialog({ editing, onClose }: { editing: Editing | null; onClose: () => void }) {
  const [name, setName] = useState(editing?.mode === "rename" ? editing.folder.name : "");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function submit(event: FormEvent) {
    event.preventDefault();
    if (!editing) return;
    setBusy(true);
    try {
      if (editing.mode === "create") await createFolder(name.trim());
      else await renameFolder(editing.folder.id, name.trim());
      invalidate();
      onClose();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={editing !== null} onOpenChange={(open) => !open && onClose()}>
      <DialogContent>
        <form onSubmit={(e) => void submit(e)} className="grid gap-5">
          <DialogHeader>
            <DialogTitle>
              {editing?.mode === "rename" ? "Переименовать папку" : "Новая папка"}
            </DialogTitle>
          </DialogHeader>
          <div className="grid gap-2">
            <Label htmlFor="folder-name">Название</Label>
            <Input
              id="folder-name"
              value={name}
              maxLength={64}
              autoFocus
              autoComplete="off"
              onChange={(e) => {
                setName(e.target.value);
                setError(null);
              }}
              aria-invalid={error !== null}
            />
            {error && <p className="text-xs text-destructive">{error}</p>}
          </div>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={onClose}>
              Отмена
            </Button>
            <Button type="submit" disabled={busy || !name.trim()}>
              {editing?.mode === "rename" ? "Сохранить" : "Создать"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

export function FoldersPage() {
  const { data: folders } = useVaultQuery("folders", listFolders);
  const [editing, setEditing] = useState<Editing | null>(null);
  const [deleting, setDeleting] = useState<FolderView | null>(null);

  async function confirmDelete() {
    if (!deleting) return;
    try {
      await deleteFolder(deleting.id);
      invalidate();
      toast.success("Папка удалена", { description: "Записи из неё остались без папки" });
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 px-8 py-8">
      <PageHeader
        title="Папки"
        description="Названия папок тоже зашифрованы"
        actions={
          <Button onClick={() => setEditing({ mode: "create" })}>
            <FolderPlusIcon />
            Новая папка
          </Button>
        }
      />

      {folders && folders.length === 0 ? (
        <EmptyState
          icon={FolderIcon}
          title="Папок пока нет"
          description="Группируйте записи: «Работа», «Банки», «Семья»…"
          action={
            <Button variant="outline" onClick={() => setEditing({ mode: "create" })}>
              <FolderPlusIcon />
              Создать папку
            </Button>
          }
        />
      ) : (
        <div className="divide-y divide-border rounded-lg border border-border bg-card">
          {folders?.map((folder) => (
            <div key={folder.id} className="flex items-center gap-1 pr-2">
              <Link
                to={`/folders/${folder.id}`}
                className="flex min-w-0 flex-1 items-center gap-3 px-4 py-3 outline-none hover:text-brand focus-visible:text-brand"
              >
                <FolderIcon className="size-4 shrink-0 text-muted-foreground" strokeWidth={1.75} />
                <span className="truncate text-sm font-medium">{folder.name}</span>
                <span className="ml-auto shrink-0 text-xs text-muted-foreground">
                  {folder.itemCount} {plural(folder.itemCount, ITEMS_FORMS)}
                </span>
              </Link>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    aria-label={`Действия с папкой ${folder.name}`}
                  >
                    <MoreHorizontalIcon />
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem onSelect={() => setEditing({ mode: "rename", folder })}>
                    <PencilIcon />
                    Переименовать
                  </DropdownMenuItem>
                  <DropdownMenuItem variant="destructive" onSelect={() => setDeleting(folder)}>
                    <Trash2Icon />
                    Удалить
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          ))}
        </div>
      )}

      {/* keyed so the input starts from the right name every time */}
      <FolderDialog
        key={editing ? (editing.mode === "rename" ? editing.folder.id : "new") : "closed"}
        editing={editing}
        onClose={() => setEditing(null)}
      />
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title="Удалить папку?"
        description={`Папка «${deleting?.name ?? ""}» будет удалена. Записи из неё не удалятся — они останутся без папки.`}
        confirmLabel="Удалить папку"
        destructive
        onConfirm={confirmDelete}
      />
    </div>
  );
}
