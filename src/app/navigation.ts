import {
  CreditCardIcon,
  FolderIcon,
  IdCardIcon,
  KeyRoundIcon,
  LayersIcon,
  LayoutDashboardIcon,
  DicesIcon,
  SettingsIcon,
  ShieldCheckIcon,
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
      { to: "/health", label: "Проверка паролей", icon: ShieldCheckIcon },
    ],
  },
  {
    id: "types",
    items: [
      { to: "/passwords", label: "Пароли", icon: KeyRoundIcon },
      { to: "/notes", label: "Заметки", icon: StickyNoteIcon },
      { to: "/cards", label: "Карты", icon: CreditCardIcon },
      { to: "/documents", label: "Документы", icon: IdCardIcon },
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
  {
    id: "tools",
    items: [{ to: "/generator", label: "Генератор", icon: DicesIcon }],
  },
];

export const SETTINGS_NAV: NavItem = { to: "/settings", label: "Настройки", icon: SettingsIcon };
