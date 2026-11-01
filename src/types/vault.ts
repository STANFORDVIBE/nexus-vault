// Mirrors the serde types in src-tauri/src/core. Keep in sync with Rust.

export type VaultState = "uninitialized" | "locked" | "unlocked" | "corrupted";

export interface VaultStatus {
  state: VaultState;
  corruptedItems: number;
}

export type ItemType = "login" | "secure_note";

export interface LoginPayload {
  title: string;
  website?: string | null;
  username?: string | null;
  email?: string | null;
  password?: string | null;
  notes?: string | null;
  tags: string[];
}

export interface SecureNotePayload {
  title: string;
  content: string;
  tags: string[];
}

export type ItemContent =
  { type: "login"; payload: LoginPayload } | { type: "secure_note"; payload: SecureNotePayload };

export type ItemInput = ItemContent & { folderId?: string | null; favorite?: boolean };

export interface ItemSummary {
  id: string;
  type: ItemType;
  title: string;
  subtitle: string | null;
  website: string | null;
  favorite: boolean;
  folderId: string | null;
  tags: string[];
  createdAt: number;
  updatedAt: number;
  deletedAt: number | null;
}

/** Login detail never carries the password itself, only whether one exists. */
export interface LoginView {
  title: string;
  website: string | null;
  username: string | null;
  email: string | null;
  notes: string | null;
  tags: string[];
  hasPassword: boolean;
}

interface ItemDetailBase {
  id: string;
  favorite: boolean;
  folderId: string | null;
  createdAt: number;
  updatedAt: number;
  deletedAt: number | null;
}

export type ItemDetail = ItemDetailBase &
  ({ type: "login"; payload: LoginView } | { type: "secure_note"; payload: SecureNotePayload });

export type ItemView = "all" | "logins" | "notes" | "favorites" | "trash" | "folder";
export type ItemSort = "updated" | "created" | "title";

export interface ItemFilter {
  view: ItemView;
  folderId?: string | null;
  query?: string;
  sort?: ItemSort;
}

export type CopyField = "username" | "email" | "password" | "website" | "content";

export interface FolderView {
  id: string;
  name: string;
  itemCount: number;
  createdAt: number;
}

export interface DashboardData {
  total: number;
  logins: number;
  notes: number;
  favorites: number;
  trash: number;
  recent: ItemSummary[];
  favoriteItems: ItemSummary[];
}

export type CloseBehavior = "lock_to_tray" | "minimize_to_tray" | "exit";

export interface Settings {
  autoLockMinutes: number;
  clipboardClearSeconds: number;
  lockOnSystemLock: boolean;
  closeBehavior: CloseBehavior;
  launchAtStartup: boolean;
}

export type GeneratorMode = "password" | "passphrase";

export interface GeneratorOptions {
  mode: GeneratorMode;
  length: number;
  uppercase: boolean;
  lowercase: boolean;
  digits: boolean;
  symbols: boolean;
  excludeAmbiguous: boolean;
  words: number;
  separator: string;
  capitalize: boolean;
  includeNumber: boolean;
}

export interface GeneratedPassword {
  value: string;
  entropyBits: number;
}

export interface BackupInfo {
  fileName: string;
  createdAt: number;
  sizeBytes: number;
}

export interface RestoreSummary {
  items: number;
  folders: number;
}
