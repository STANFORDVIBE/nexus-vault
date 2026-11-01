import {
  FolderIcon,
  KeyRoundIcon,
  LayersIcon,
  LayoutDashboardIcon,
  PaletteIcon,
  SettingsIcon,
  StarIcon,
  StickyNoteIcon,
  Trash2Icon,
  type LucideIcon,
} from "lucide-react";

export interface NavItem {
  to: string;
  label: string;
  icon: LucideIcon;
}

export interface NavSection {
  id: string;
  items: NavItem[];
}

export const NAV_SECTIONS: NavSection[] = [
  {
    id: "overview",
    items: [
      { to: "/dashboard", label: "Обзор", icon: LayoutDashboardIcon },
      { to: "/items", label: "Все записи", icon: LayersIcon },
    ],
  },
  {
    id: "types",
    items: [
      { to: "/passwords", label: "Пароли", icon: KeyRoundIcon },
      { to: "/notes", label: "Заметки", icon: StickyNoteIcon },
    ],
  },
  {
    id: "organize",
    items: [
      { to: "/favorites", label: "Избранное", icon: StarIcon },
      { to: "/folders", label: "Папки", icon: FolderIcon },
      { to: "/trash", label: "Корзина", icon: Trash2Icon },
    ],
  },
];

export const SETTINGS_NAV: NavItem = { to: "/settings", label: "Настройки", icon: SettingsIcon };

// Temporary entry for Phase 2: the component showcase. Removed once real screens land.
export const DESIGN_SYSTEM_NAV: NavItem = {
  to: "/design-system",
  label: "Дизайн-система",
  icon: PaletteIcon,
};
