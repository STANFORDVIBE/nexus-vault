import type { LucideIcon } from "lucide-react";
import { cn } from "cn";
import { ToggleGroup as ToggleGroupPrimitive } from "radix-ui";

export interface SegmentedOption<T extends string> {
  value: T;
  label: string;
  icon?: LucideIcon;
}

interface SegmentedControlProps<T extends string> {
  value: T;
  options: readonly SegmentedOption<T>[];
  onValueChange: (value: T) => void;
  "aria-label": string;
  className?: string;
}

/** Compact single-choice switcher (theme, generator mode, etc.). Always keeps one option selected. */
function SegmentedControl<T extends string>({
  value,
  options,
  onValueChange,
  className,
  "aria-label": ariaLabel,
}: SegmentedControlProps<T>) {
  return (
    <ToggleGroupPrimitive.Root
      type="single"
      value={value}
      // Radix emits "" when the active item is clicked again; ignore it
      onValueChange={(next) => next && onValueChange(next as T)}
      aria-label={ariaLabel}
      data-slot="segmented-control"
      className={cn("inline-flex h-9 items-center gap-0.5 rounded-md bg-muted p-0.5", className)}
    >
      {options.map(({ value: optionValue, label, icon: Icon }) => (
        <ToggleGroupPrimitive.Item
          key={optionValue}
          value={optionValue}
          className="inline-flex h-full items-center gap-1.5 rounded-sm px-3 text-sm font-medium text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 data-[state=on]:bg-background data-[state=on]:text-foreground data-[state=on]:shadow-xs dark:data-[state=on]:bg-elevated"
        >
          {Icon && <Icon className="size-4" />}
          {label}
        </ToggleGroupPrimitive.Item>
      ))}
    </ToggleGroupPrimitive.Root>
  );
}

export { SegmentedControl };
