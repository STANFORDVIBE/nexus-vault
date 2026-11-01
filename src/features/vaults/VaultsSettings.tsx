import { useCallback, useEffect, useState, type FormEvent } from "react";
import {
  DatabaseIcon,
  FolderOpenIcon,
  MoreHorizontalIcon,
  PencilIcon,
  PlusIcon,
  XIcon,
} from "lucide-react";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { Badge } from "@/components/ui/badge";
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
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { NewVaultDialog } from "@/features/vaults/NewVaultDialog";
import { formatRelative } from "@/lib/format";
import { errorMessage } from "@/services/errors";
import {
  forgetVault,
  listVaults,
  openVaultFile,
  renameVault,
  revealVault,
  switchVault,
} from "@/services/vault";
import { useSessionStore } from "@/store/session";
import type { VaultListEntry, VaultStatus } from "@/types/vault";

interface RenameProps {
  onClose: () => void;
  onRenamed: (status: VaultStatus) => void;
}

/** Mounted per vault (keyed), so the field starts from that vault's current name. */
function RenameForm({ vault, onClose, onRenamed }: RenameProps & { vault: VaultListEntry }) {
  const [name, setName] = useState(vault.name);
  const [error, setError] = useState<string | null>(null);

  async function submit(event: FormEvent) {
    event.preventDefault();
    try {
      onRenamed(await renameVault(vault.id, name));
      onClose();
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  return (
    <form onSubmit={(e) => void submit(e)} className="grid gap-4">
      <DialogHeader>
        <DialogTitle>Переименовать хранилище</DialogTitle>
      </DialogHeader>
      <Input
        aria-label="Название хранилища"
        value={name}
        onChange={(e) => setName(e.target.value)}
        maxLength={64}
        autoFocus
      />
      {error && <p className="text-xs text-destructive">{error}</p>}
      <DialogFooter>
        <Button type="button" variant="outline" onClick={onClose}>
          Отмена
        </Button>
        <Button type="submit" disabled={!name.trim()}>
          Сохранить
        </Button>
      </DialogFooter>
    </form>
  );
}

function RenameDialog({ vault, ...props }: RenameProps & { vault: VaultListEntry | null }) {
  return (
    <Dialog open={vault !== null} onOpenChange={(open) => !open && props.onClose()}>
      <DialogContent>
        {vault && <RenameForm key={vault.id} vault={vault} {...props} />}
      </DialogContent>
    </Dialog>
  );
}

/** Settings section: every known vault with its file location and actions. */
export function VaultsSettings() {
  const setStatus = useSessionStore((s) => s.setStatus);
  const [vaults, setVaults] = useState<VaultListEntry[]>([]);
  const [renaming, setRenaming] = useState<VaultListEntry | null>(null);
  const [forgetting, setForgetting] = useState<VaultListEntry | null>(null);
  const [creating, setCreating] = useState(false);

  const reload = useCallback(() => {
    listVaults()
      .then(setVaults)
      .catch((e: unknown) => toast.error(errorMessage(e)));
  }, []);

  useEffect(reload, [reload]);

  async function run(action: () => Promise<VaultStatus | null | void>) {
    try {
      const status = await action();
      if (status) setStatus(status);
      reload();
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  return (
    <div className="flex flex-col">
      <ul className="divide-y divide-border">
        {vaults.map((vault) => (
          <li key={vault.id} className="flex items-center gap-3 px-6 py-3">
            <DatabaseIcon className="size-4 shrink-0 text-muted-foreground" />
            <div className="flex min-w-0 flex-1 flex-col gap-0.5">
              <div className="flex items-center gap-2">
                <span className="truncate text-sm font-medium">{vault.name}</span>
                {vault.active && <Badge variant="secondary">Открыто</Badge>}
                {!vault.exists && <Badge variant="outline">Файл не найден</Badge>}
              </div>
              <span
                className="truncate font-mono text-[11px] text-muted-foreground"
                title={vault.path}
              >
                {vault.path}
              </span>
              {vault.lastOpenedAt && (
                <span className="text-[11px] text-muted-foreground">
                  Открывалось {formatRelative(vault.lastOpenedAt)}
                </span>
              )}
            </div>
            {!vault.active && (
              <Button
                variant="outline"
                size="sm"
                onClick={() => void run(() => switchVault(vault.id))}
              >
                Открыть
              </Button>
            )}
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  aria-label={`Действия с хранилищем ${vault.name}`}
                >
                  <MoreHorizontalIcon />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end" className="w-56">
                <DropdownMenuItem onSelect={() => setRenaming(vault)}>
                  <PencilIcon />
                  Переименовать
                </DropdownMenuItem>
                <DropdownMenuItem onSelect={() => void run(() => revealVault(vault.id))}>
                  <FolderOpenIcon />
                  Показать в папке
                </DropdownMenuItem>
                {vaults.length > 1 && (
                  <>
                    <DropdownMenuSeparator />
                    <DropdownMenuItem variant="destructive" onSelect={() => setForgetting(vault)}>
                      <XIcon />
                      Убрать из списка
                    </DropdownMenuItem>
                  </>
                )}
              </DropdownMenuContent>
            </DropdownMenu>
          </li>
        ))}
      </ul>
      <div className="flex flex-wrap gap-2 border-t border-border px-6 py-4">
        <Button variant="outline" onClick={() => setCreating(true)}>
          <PlusIcon />
          Создать новое
        </Button>
        <Button variant="outline" onClick={() => void run(openVaultFile)}>
          <FolderOpenIcon />
          Открыть файл…
        </Button>
      </div>

      <NewVaultDialog open={creating} onOpenChange={setCreating} />
      <RenameDialog
        vault={renaming}
        onClose={() => setRenaming(null)}
        onRenamed={(status) => {
          setStatus(status);
          reload();
        }}
      />
      <ConfirmDialog
        open={forgetting !== null}
        onOpenChange={(open) => !open && setForgetting(null)}
        title="Убрать хранилище из списка?"
        description={
          <>
            Хранилище «{forgetting?.name}» пропадёт из списка, но файл останется на диске. Его
            всегда можно вернуть через «Открыть файл…».
          </>
        }
        confirmLabel="Убрать"
        onConfirm={() => forgetting && run(() => forgetVault(forgetting.id))}
      />
    </div>
  );
}
