import { useEffect, useState } from "react";
import { FieldRow } from "@/features/items/FieldRow";
import { groupCode } from "@/lib/format";
import { getTotpCode } from "@/services/vault";
import type { TotpCode } from "@/types/vault";

interface TotpRowProps {
  id: string;
  onCopy?: () => void;
}

/**
 * Current two-factor code with a countdown. A new code is asked from Rust only when
 * the old one expires; the countdown in between is local.
 */
export function TotpRow({ id, onCopy }: TotpRowProps) {
  const [totp, setTotp] = useState<TotpCode | null>(null);
  const [failed, setFailed] = useState(false);
  const [left, setLeft] = useState(0);

  useEffect(() => {
    let alive = true;
    let timer = 0;
    async function load() {
      try {
        const next = await getTotpCode(id);
        if (!alive) return;
        setTotp(next);
        setLeft(next.remaining);
        setFailed(false);
      } catch {
        if (alive) setFailed(true);
      }
    }
    void load();
    timer = window.setInterval(() => {
      setLeft((s) => {
        if (s <= 1) {
          void load();
          return 0;
        }
        return s - 1;
      });
    }, 1000);
    return () => {
      alive = false;
      window.clearInterval(timer);
    };
  }, [id]);

  const period = totp?.period ?? 30;
  return (
    <FieldRow label="Код 2FA" mono onCopy={onCopy}>
      {failed ? (
        <span className="text-muted-foreground">Не удалось получить код</span>
      ) : totp ? (
        <span className="flex items-center gap-3">
          <span className="text-base tracking-[0.15em] select-text">{groupCode(totp.code)}</span>
          <span
            className="h-1 w-16 overflow-hidden rounded-full bg-muted"
            role="progressbar"
            aria-label={`Код сменится через ${left} с`}
            aria-valuemin={0}
            aria-valuemax={period}
            aria-valuenow={left}
          >
            <span
              className={left <= 5 ? "block h-full bg-destructive" : "block h-full bg-brand"}
              style={{ width: `${(left / period) * 100}%` }}
            />
          </span>
          <span className="text-xs text-muted-foreground tabular-nums">{left} с</span>
        </span>
      ) : (
        <span className="text-muted-foreground">…</span>
      )}
    </FieldRow>
  );
}
