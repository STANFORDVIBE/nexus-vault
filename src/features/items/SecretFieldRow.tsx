import { useEffect, useState, type ReactNode } from "react";
import { EyeIcon, EyeOffIcon } from "lucide-react";
import { toast } from "sonner";
import { FieldRow, IconAction } from "@/features/items/FieldRow";
import { useCopy } from "@/hooks/useCopy";
import { errorMessage } from "@/services/errors";
import { revealField } from "@/services/vault";
import type { CopyField } from "@/types/vault";

/** A revealed value is hidden again automatically after this long. */
const REVEAL_MS = 30_000;

interface SecretFieldRowProps {
  itemId: string;
  field: CopyField;
  label: string;
  readOnly: boolean;
  /** What to show while hidden (default: dots). */
  masked?: ReactNode;
  /** How to display the revealed value (e.g. card number in groups of four). */
  format?: (value: string) => string;
}

/** Masked value fetched from Rust only on "show"; copying never goes through JS. */
export function SecretFieldRow({
  itemId,
  field,
  label,
  readOnly,
  masked,
  format,
}: SecretFieldRowProps) {
  const { copyField } = useCopy();
  const [revealed, setRevealed] = useState<string | null>(null);

  useEffect(() => {
    if (revealed === null) return;
    const timer = window.setTimeout(() => setRevealed(null), REVEAL_MS);
    return () => window.clearTimeout(timer);
  }, [revealed]);

  async function toggle() {
    if (revealed !== null) {
      setRevealed(null);
      return;
    }
    try {
      setRevealed(await revealField(itemId, field));
    } catch (e) {
      toast.error(errorMessage(e));
    }
  }

  return (
    <FieldRow
      label={label}
      mono
      onCopy={readOnly ? undefined : () => void copyField(itemId, field)}
      actions={
        !readOnly && (
          <IconAction
            label={revealed !== null ? "Скрыть" : "Показать"}
            icon={revealed !== null ? EyeOffIcon : EyeIcon}
            onClick={() => void toggle()}
            pressed={revealed !== null}
          />
        )
      }
    >
      {revealed !== null ? (
        <span className="select-text">{format ? format(revealed) : revealed}</span>
      ) : (
        (masked ?? <span className="tracking-[0.2em]">••••••••</span>)
      )}
    </FieldRow>
  );
}
