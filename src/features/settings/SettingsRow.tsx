import type { ReactNode } from "react";
import { Label } from "@/components/ui/label";

/** One setting: label and hint on the left, the control on the right. */
export function SettingsRow({
  label,
  description,
  htmlFor,
  children,
}: {
  label: string;
  description?: ReactNode;
  htmlFor?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex flex-wrap items-center justify-between gap-x-6 gap-y-2 px-6 py-4">
      <div className="flex min-w-0 flex-1 basis-60 flex-col gap-0.5">
        <Label htmlFor={htmlFor} className="text-sm">
          {label}
        </Label>
        {description && (
          <p className="text-xs leading-relaxed text-muted-foreground">{description}</p>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}
