import { NavLink } from "react-router";
import { cn } from "cn";
import { FolderIcon } from "lucide-react";
import { NAV_SECTIONS, SETTINGS_NAV } from "@/app/navigation";
import { Logo } from "@/components/common/Logo";
import { Separator } from "@/components/ui/separator";
import { SidebarNavLink } from "@/components/layout/SidebarNavLink";
import { VaultStatus } from "@/components/layout/VaultStatus";
import { useVaultQuery } from "@/hooks/useVaultQuery";
import { listFolders } from "@/services/vault";

function FolderLinks() {
  const { data: folders } = useVaultQuery("folders", listFolders);
  if (!folders?.length) return null;

  return (
    <div className="flex flex-col gap-0.5">
      <div className="px-2.5 pb-1 text-[11px] font-medium tracking-wide text-muted-foreground/80 uppercase">
        Папки
      </div>
      {folders.map((folder) => (
        <NavLink
          key={folder.id}
          to={`/folders/${folder.id}`}
          className={({ isActive }) =>
            cn(
              "flex h-7 items-center gap-2.5 rounded-md px-2.5 text-[13px] text-muted-foreground transition-colors outline-none",
              "hover:bg-sidebar-accent hover:text-sidebar-foreground focus-visible:ring-2 focus-visible:ring-sidebar-ring/50",
              isActive && "bg-sidebar-accent text-sidebar-foreground",
            )
          }
        >
          <FolderIcon className="size-3.5 shrink-0" strokeWidth={1.75} />
          <span className="min-w-0 flex-1 truncate">{folder.name}</span>
          <span className="text-[11px] text-muted-foreground/70 tabular-nums">
            {folder.itemCount}
          </span>
        </NavLink>
      ))}
    </div>
  );
}

export function Sidebar() {
  return (
    <aside className="flex h-full w-[232px] shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
      <div className="flex h-14 shrink-0 items-center px-4">
        <Logo />
      </div>

      <nav
        aria-label="Основная навигация"
        className="flex flex-1 flex-col gap-4 overflow-y-auto px-2 py-2"
      >
        {NAV_SECTIONS.map((section) => (
          <div key={section.id} className="flex flex-col gap-0.5">
            {section.items.map((item) => (
              <SidebarNavLink key={item.to} {...item} />
            ))}
          </div>
        ))}

        <FolderLinks />

        <div className="mt-auto flex flex-col gap-0.5">
          <SidebarNavLink {...SETTINGS_NAV} />
        </div>
      </nav>

      <Separator className="bg-sidebar-border" />
      <VaultStatus />
    </aside>
  );
}
