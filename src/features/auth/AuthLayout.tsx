import type { ReactNode } from "react";
import { LogoMark } from "@/components/common/Logo";

interface AuthLayoutProps {
  title: string;
  description?: ReactNode;
  children: ReactNode;
  footer?: ReactNode;
}

/** Centered card used by the onboarding, unlock and recovery screens. */
export function AuthLayout({ title, description, children, footer }: AuthLayoutProps) {
  return (
    <div className="flex h-full overflow-y-auto bg-background">
      <div className="m-auto flex w-full max-w-[400px] flex-col items-center gap-6 px-6 py-10">
        <div className="flex flex-col items-center gap-4 text-center">
          <div className="flex size-14 items-center justify-center rounded-xl border border-border bg-surface">
            <LogoMark className="size-8" />
          </div>
          <div className="flex flex-col gap-1.5">
            <h1 className="text-xl">{title}</h1>
            {description && (
              <p className="text-sm leading-relaxed text-muted-foreground">{description}</p>
            )}
          </div>
        </div>
        <div className="w-full">{children}</div>
        {footer && <div className="flex flex-col items-center gap-2">{footer}</div>}
      </div>
    </div>
  );
}
