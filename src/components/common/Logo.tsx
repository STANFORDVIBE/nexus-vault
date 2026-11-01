import { cn } from "cn";

/** Nexus Vault mark: a hexagonal vault with a keyhole. Same geometry as the app icon. */
export function LogoMark({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 32 32"
      fill="none"
      aria-hidden="true"
      className={cn("size-7 text-brand", className)}
    >
      <path
        d="M16 3.5 26.8 9.75v12.5L16 28.5 5.2 22.25V9.75L16 3.5Z"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinejoin="round"
      />
      <circle cx="16" cy="14" r="3" fill="currentColor" />
      <path d="M14.6 15.5h2.8l.8 5.5h-4.4l.8-5.5Z" fill="currentColor" />
    </svg>
  );
}

export function Logo({ className }: { className?: string }) {
  return (
    <div className={cn("flex items-center gap-2.5", className)}>
      <LogoMark />
      <span className="text-[15px] font-semibold tracking-tight">Nexus Vault</span>
    </div>
  );
}
