import { NavLink } from "react-router";
import { cn } from "cn";
import type { NavItem } from "@/app/navigation";

export function SidebarNavLink({ to, label, icon: Icon }: NavItem) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        cn(
          "group flex h-8 items-center gap-2.5 rounded-md px-2.5 text-[13px] font-medium text-muted-foreground transition-colors outline-none",
          "hover:bg-sidebar-accent hover:text-sidebar-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring/50",
          isActive && "bg-sidebar-accent text-sidebar-foreground",
        )
      }
    >
      {({ isActive }) => (
        <>
          <Icon
            className={cn("size-4 shrink-0", isActive && "text-brand")}
            strokeWidth={isActive ? 2 : 1.75}
          />
          <span className="truncate">{label}</span>
        </>
      )}
    </NavLink>
  );
}
