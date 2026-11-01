import { DESIGN_SYSTEM_NAV, NAV_SECTIONS, SETTINGS_NAV } from "@/app/navigation";
import { Logo } from "@/components/common/Logo";
import { Separator } from "@/components/ui/separator";
import { SidebarNavLink } from "@/components/layout/SidebarNavLink";
import { VaultStatus } from "@/components/layout/VaultStatus";

export function Sidebar() {
  return (
    <aside className="flex h-full w-[232px] shrink-0 flex-col border-r border-sidebar-border bg-sidebar text-sidebar-foreground">
      <div className="flex h-14 items-center px-4">
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

        <div className="mt-auto flex flex-col gap-0.5">
          <SidebarNavLink {...DESIGN_SYSTEM_NAV} />
          <SidebarNavLink {...SETTINGS_NAV} />
        </div>
      </nav>

      <Separator className="bg-sidebar-border" />
      <VaultStatus />
    </aside>
  );
}
