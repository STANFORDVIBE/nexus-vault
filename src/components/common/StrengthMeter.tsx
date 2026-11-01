import { cn } from "cn";
import type { StrengthLevel } from "@/lib/password-strength";

const COLORS: Record<StrengthLevel, string> = {
  0: "bg-destructive",
  1: "bg-destructive",
  2: "bg-amber-500",
  3: "bg-success",
  4: "bg-success",
};

interface StrengthMeterProps {
  level: StrengthLevel;
  label: string;
  hint?: string;
  className?: string;
}

export function StrengthMeter({ level, label, hint, className }: StrengthMeterProps) {
  return (
    <div className={cn("flex flex-col gap-1.5", className)}>
      <div
        className="flex gap-1"
        role="meter"
        aria-valuemin={0}
        aria-valuemax={4}
        aria-valuenow={level}
        aria-label="Надёжность пароля"
      >
        {[1, 2, 3, 4].map((step) => (
          <span
            key={step}
            className={cn(
              "h-1 flex-1 rounded-full bg-muted transition-colors",
              level >= step && COLORS[level],
            )}
          />
        ))}
      </div>
      <div className="flex justify-between text-xs text-muted-foreground">
        <span>{label}</span>
        {hint && <span>{hint}</span>}
      </div>
    </div>
  );
}
