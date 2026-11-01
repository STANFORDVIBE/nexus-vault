import { useState } from "react";
import { cn } from "cn";
import {
  CheckIcon,
  ChevronsUpDownIcon,
  DatabaseIcon,
  FolderOpenIcon,
  PlusIcon,
  SettingsIcon,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { NewVaultDialog } from "@/features/vaults/NewVaultDialog";
import { errorMessage } from "@/services/errors";
import { listVaults, openVaultFile, switchVault } from "@/services/vault";
import { useSessionStore } from "@/store/session";
import type { VaultListEntry } from "@/types/vault";

interface VaultSwitcherProps {
  /** "sidebar": full-width button with the vault name; "auth": small link-style button. */
  variant: "sidebar" | "auth";
  /** Shown in the sidebar menu only: jump to the vault list in settings. */
  onManage?: () => void;
}

/** Menu to switch between vaults, create a new one or open an existing file. */
export function VaultSwitcher({ variant, onManage }: VaultSwitcherProps) {
  const status = useSessionStore((s) => s.status);
  const setStatus = useSessionStore((s) => s.setStatus);
  const [vaults, setVaults] = useState<VaultListEntry[]>([]);
  const [creating, setCreating] = useState(false);

  async function load(open: boolean) {
    if (!open) return;
    try {
      setVaults(await listVaults());
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  async function run(action: () => Promise<Parameters<typeof setStatus>[0] | null>) {
    try {
      const next = await action();
      if (next) setStatus(next);
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  const name = status?.vault.name ?? "";

  return (
    <>
      <DropdownMenu onOpenChange={(open) => void load(open)}>
        <DropdownMenuTrigger asChild>
          {variant === "sidebar" ? (
            <button
              type="button"
              className="flex h-9 w-full items-center gap-2 rounded-md border border-sidebar-border bg-sidebar-accent/40 px-2.5 text-left text-[13px] transition-colors outline-none hover:bg-sidebar-accent focus-visible:ring-2 focus-visible:ring-sidebar-ring/50"
              aria-label={`Хранилище: ${name}. Сменить хранилище`}
            >
              <DatabaseIcon className="size-3.5 shrink-0 text-muted-foreground" />
              <span className="min-w-0 flex-1 truncate font-medium">{name}</span>
              <ChevronsUpDownIcon className="size-3.5 shrink-0 text-muted-foreground" />
            </button>
          ) : (
            <Button variant="ghost" size="sm" className="text-muted-foreground">
              <DatabaseIcon />
              {(status?.vaultCount ?? 1) > 1 ? "Другое хранилище" : "Хранилища"}
            </Button>
          )}
        </DropdownMenuTrigger>
        <DropdownMenuContent
          align={variant === "sidebar" ? "start" : "center"}
          className={cn(variant === "sidebar" ? "w-64" : "w-72")}
        >
          <DropdownMenuLabel className="text-xs text-muted-foreground">Хранилища</DropdownMenuLabel>
          {vaults.map((vault) => (
            <DropdownMenuItem
              key={vault.id}
              onSelect={() => !vault.active && void run(() => switchVault(vault.id))}
              className="gap-2"
            >
              <CheckIcon className={cn("size-4", !vault.active && "invisible")} />
              <span className="min-w-0 flex-1 truncate">{vault.name}</span>
              {!vault.exists && (
                <span className="text-[11px] text-muted-foreground">не создано</span>
              )}
            </DropdownMenuItem>
          ))}
          <DropdownMenuSeparator />
          <DropdownMenuItem onSelect={() => setCreating(true)}>
            <PlusIcon />
            Создать новое хранилище…
          </DropdownMenuItem>
          <DropdownMenuItem onSelect={() => void run(openVaultFile)}>
            <FolderOpenIcon />
            Открыть файл хранилища…
          </DropdownMenuItem>
          {onManage && (
            <DropdownMenuItem onSelect={onManage}>
              <SettingsIcon />
              Управление хранилищами
            </DropdownMenuItem>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
      <NewVaultDialog open={creating} onOpenChange={setCreating} />
    </>
  );
}
