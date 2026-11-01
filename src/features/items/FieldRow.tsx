import type { ReactNode } from "react";
import { CopyIcon } from "lucide-react";
import { cn } from "cn";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

interface FieldRowProps {
  label: string;
  children: ReactNode;
  onCopy?: () => void;
  actions?: ReactNode;
  mono?: boolean;
}

/** One labelled value in the item detail card, with hover actions on the right. */
export function FieldRow({ label, children, onCopy, actions, mono }: FieldRowProps) {
  return (
    <div className="group flex items-center gap-3 px-4 py-3">
      <div className="min-w-0 flex-1">
        <div className="text-xs text-muted-foreground">{label}</div>
        <div className={cn("mt-0.5 truncate text-sm", mono && "font-mono tracking-wide")}>
          {children}
        </div>
      </div>
      <div className="flex shrink-0 items-center gap-0.5">
        {actions}
        {onCopy && <IconAction label="Копировать" icon={CopyIcon} onClick={onCopy} />}
      </div>
    </div>
  );
}

export function IconAction({
  label,
  icon: Icon,
  onClick,
  pressed,
}: {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  onClick: () => void;
  pressed?: boolean;
}) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon-sm"
          onClick={onClick}
          aria-label={label}
          aria-pressed={pressed}
          className="text-muted-foreground hover:text-foreground"
        >
          <Icon />
        </Button>
      </TooltipTrigger>
      <TooltipContent>{label}</TooltipContent>
    </Tooltip>
  );
}
