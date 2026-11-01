import { useLocation } from "react-router";
import { ConstructionIcon } from "lucide-react";
import { EmptyState } from "@/components/common/EmptyState";
import { NAV_SECTIONS, SETTINGS_NAV } from "@/app/navigation";

const LABELS = new Map(
  [...NAV_SECTIONS.flatMap((section) => section.items), SETTINGS_NAV].map((item) => [
    item.to,
    item.label,
  ]),
);

/** Stand-in for sections that arrive in Phase 3 (Main UI). */
export function SectionPlaceholder() {
  const { pathname } = useLocation();
  const label = LABELS.get(pathname) ?? "Раздел";

  return (
    <EmptyState
      icon={ConstructionIcon}
      title={`${label}: ещё в разработке`}
      description="Этот экран появится на этапе Main UI. Сейчас готова дизайн-система."
      className="h-full"
    />
  );
}
