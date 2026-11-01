import { useEffect, useState, type FormEvent, type ReactNode } from "react";
import { ConfirmDialog } from "@/components/common/ConfirmDialog";
import { Button } from "@/components/ui/button";

interface FormShellProps {
  title: string;
  dirty: boolean;
  busy: boolean;
  onSubmit: (event: FormEvent) => unknown;
  onCancel: () => void;
  headerAction?: ReactNode;
  children: ReactNode;
  submitLabel?: string;
}

/**
 * Layout for the detail-pane editors: sticky header with Cancel / Save,
 * Ctrl+S to save, Escape to cancel (asks first when there are unsaved changes).
 */
export function FormShell({
  title,
  dirty,
  busy,
  onSubmit,
  onCancel,
  headerAction,
  children,
  submitLabel = "Сохранить",
}: FormShellProps) {
  const [confirmDiscard, setConfirmDiscard] = useState(false);

  function cancel() {
    if (dirty) setConfirmDiscard(true);
    else onCancel();
  }

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent) {
      if (event.defaultPrevented) return;
      // Radix closes popovers/selects/dialogs on the same Escape press; leave those alone
      const layerOpen = document.querySelector(
        "[role=dialog],[role=alertdialog],[data-radix-popper-content-wrapper]",
      );
      if (event.key === "Escape" && !layerOpen) {
        event.preventDefault();
        if (dirty) setConfirmDiscard(true);
        else onCancel();
      } else if ((event.ctrlKey || event.metaKey) && event.code === "KeyS") {
        event.preventDefault();
        document.querySelector<HTMLFormElement>("form[data-item-form]")?.requestSubmit();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [dirty, onCancel]);

  return (
    <form
      data-item-form
      onSubmit={(e) => void onSubmit(e)}
      className="flex h-full flex-col"
      noValidate
    >
      <div className="flex h-14 shrink-0 items-center justify-between gap-3 border-b border-border px-6">
        <div className="flex min-w-0 items-center gap-1">
          <h2 className="truncate text-base">{title}</h2>
          {headerAction}
        </div>
        <div className="flex shrink-0 gap-2">
          <Button type="button" variant="outline" onClick={cancel} disabled={busy}>
            Отмена
          </Button>
          <Button type="submit" disabled={busy}>
            {busy ? "Сохраняем…" : submitLabel}
          </Button>
        </div>
      </div>
      <div className="flex-1 overflow-y-auto">
        <div className="mx-auto flex max-w-2xl flex-col gap-5 px-6 py-6">{children}</div>
      </div>

      <ConfirmDialog
        open={confirmDiscard}
        onOpenChange={setConfirmDiscard}
        title="Отменить изменения?"
        description="Несохранённые изменения будут потеряны."
        confirmLabel="Не сохранять"
        destructive
        onConfirm={onCancel}
      />
    </form>
  );
}
