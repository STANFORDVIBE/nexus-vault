import { useState } from "react";
import { ChevronDownIcon, HistoryIcon } from "lucide-react";
import { toast } from "sonner";
import { cn } from "cn";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { Button } from "@/components/ui/button";
import { SecretFieldRow } from "@/features/items/SecretFieldRow";
import { formatDateTime } from "@/lib/format";
import { errorMessage } from "@/services/errors";
import { clearPasswordHistory } from "@/services/vault";
import { invalidate } from "@/store/session";

interface PasswordHistoryProps {
  id: string;
  history: { replacedAt: number }[];
  readOnly: boolean;
}

/** Previous passwords of a login: collapsed by default, each revealed on request. */
export function PasswordHistory({ id, history, readOnly }: PasswordHistoryProps) {
  const [open, setOpen] = useState(false);
  const [clearing, setClearing] = useState(false);

  if (history.length === 0) return null;

  async function clear() {
    try {
      await clearPasswordHistory(id);
      invalidate();
      toast.success("История паролей очищена");
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  return (
    <div className="flex flex-col gap-2">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        aria-expanded={open}
        className="flex items-center gap-2 self-start rounded-md text-xs font-medium text-muted-foreground outline-none hover:text-foreground focus-visible:text-foreground"
      >
        <HistoryIcon className="size-3.5" />
        Старые пароли · {history.length}
        <ChevronDownIcon className={cn("size-3.5 transition-transform", open && "rotate-180")} />
      </button>
      {open && (
        <>
          <div className="divide-y divide-border rounded-lg border border-border bg-card">
            {history.map((entry, index) => (
              <SecretFieldRow
                key={`${entry.replacedAt}-${index}`}
                itemId={id}
                field={`history:${index}`}
                label={`Заменён ${formatDateTime(entry.replacedAt)}`}
                readOnly={readOnly}
              />
            ))}
          </div>
          {!readOnly && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              className="self-start text-muted-foreground"
              onClick={() => setClearing(true)}
            >
              Очистить историю
            </Button>
          )}
        </>
      )}
      <ConfirmDialog
        open={clearing}
        onOpenChange={setClearing}
        title="Очистить историю паролей?"
        description="Старые пароли этой записи будут удалены без возможности восстановления. Текущий пароль останется."
        confirmLabel="Очистить"
        destructive
        onConfirm={clear}
      />
    </div>
  );
}
