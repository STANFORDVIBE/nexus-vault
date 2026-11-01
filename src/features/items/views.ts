import {
  KeyRoundIcon,
  LayersIcon,
  StarIcon,
  StickyNoteIcon,
  Trash2Icon,
  FolderOpenIcon,
  type LucideIcon,
} from "lucide-react";
import type { ItemType, ItemView } from "@/types/vault";

export interface ViewConfig {
  view: ItemView;
  title: string;
  /** Item type pre-selected by the "Добавить" button, if the view implies one. */
  createType?: ItemType;
  empty: { icon: LucideIcon; title: string; description: string };
}

export const VIEWS: Record<Exclude<ItemView, "folder">, ViewConfig> = {
  all: {
    view: "all",
    title: "Все записи",
    empty: {
      icon: LayersIcon,
      title: "Пока пусто",
      description: "Добавьте первый пароль или защищённую заметку",
    },
  },
  logins: {
    view: "logins",
    title: "Пароли",
    createType: "login",
    empty: {
      icon: KeyRoundIcon,
      title: "Нет сохранённых паролей",
      description: "Сохраните логин и пароль от любимого сайта",
    },
  },
  notes: {
    view: "notes",
    title: "Заметки",
    createType: "secure_note",
    empty: {
      icon: StickyNoteIcon,
      title: "Нет заметок",
      description: "Храните коды восстановления, ключи и другие секреты",
    },
  },
  favorites: {
    view: "favorites",
    title: "Избранное",
    empty: {
      icon: StarIcon,
      title: "В избранном пусто",
      description: "Отмечайте звёздочкой записи, которые нужны чаще всего",
    },
  },
  trash: {
    view: "trash",
    title: "Корзина",
    empty: {
      icon: Trash2Icon,
      title: "Корзина пуста",
      description: "Удалённые записи попадают сюда, пока вы не очистите корзину",
    },
  },
};

export const FOLDER_EMPTY = {
  icon: FolderOpenIcon,
  title: "В папке пусто",
  description: "Добавьте запись или переместите сюда существующую",
};
