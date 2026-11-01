// Mirrors the serde types in src-tauri/src/core. Keep in sync with Rust.

export type VaultState = "uninitialized" | "locked" | "unlocked" | "corrupted";

export interface ActiveVault {
  id: string;
  name: string;
  path: string;
}

export interface VaultStatus {
  state: VaultState;
  corruptedItems: number;
  vault: ActiveVault;
  /** Registered vaults; switching is offered when there is more than one. */
  vaultCount: number;
}

export interface VaultListEntry {
  id: string;
  name: string;
  path: string;
  lastOpenedAt: number | null;
  /** False when the file was moved or deleted outside the app. */
  exists: boolean;
  active: boolean;
}

export type ItemType = "login" | "secure_note" | "card" | "identity";

/** Extra user-defined field. `hidden` ones are masked and copied by Rust only. */
export interface CustomField {
  label: string;
  value: string;
  hidden: boolean;
}

/** Custom field in a detail view: hidden values are not sent (`value` is null). */
export interface FieldView {
  label: string;
  value: string | null;
  hidden: boolean;
}

export interface LoginPayload {
  title: string;
  website?: string | null;
  username?: string | null;
  email?: string | null;
  password?: string | null;
  notes?: string | null;
  tags: string[];
  fields?: CustomField[];
}

export interface CardPayload {
  title: string;
  cardholder?: string | null;
  number?: string | null;
  /** MM/YY */
  expiry?: string | null;
  cvv?: string | null;
  pin?: string | null;
  notes?: string | null;
  tags: string[];
  fields?: CustomField[];
}

export interface IdentityPayload {
  title: string;
  kind?: string | null;
  fullName?: string | null;
  number?: string | null;
  issuedBy?: string | null;
  issueDate?: string | null;
  expiryDate?: string | null;
  notes?: string | null;
  tags: string[];
  fields?: CustomField[];
}

export interface SecureNotePayload {
  title: string;
  content: string;
  tags: string[];
}

export type ItemContent =
  | { type: "login"; payload: LoginPayload }
  | { type: "secure_note"; payload: SecureNotePayload }
  | { type: "card"; payload: CardPayload }
  | { type: "identity"; payload: IdentityPayload };

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
  fields: FieldView[];
  passwordChangedAt: number | null;
}

/** Card detail: number, CVV and PIN stay in Rust (only `•••• 1234` is shown). */
export interface CardView {
  title: string;
  cardholder: string | null;
  numberMasked: string | null;
  expiry: string | null;
  hasCvv: boolean;
  hasPin: boolean;
  notes: string | null;
  tags: string[];
  fields: FieldView[];
}

export interface IdentityView {
  title: string;
  kind: string | null;
  fullName: string | null;
  hasNumber: boolean;
  issuedBy: string | null;
  issueDate: string | null;
  expiryDate: string | null;
  notes: string | null;
  tags: string[];
  fields: FieldView[];
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
  (
    | { type: "login"; payload: LoginView }
    | { type: "secure_note"; payload: SecureNotePayload }
    | { type: "card"; payload: CardView }
    | { type: "identity"; payload: IdentityView }
  );

export type ItemView =
  "all" | "logins" | "notes" | "cards" | "identities" | "favorites" | "trash" | "folder";
export type ItemSort = "updated" | "created" | "title";

export interface ItemFilter {
  view: ItemView;
  folderId?: string | null;
  query?: string;
  sort?: ItemSort;
}

export type CopyField =
  | "username"
  | "email"
  | "password"
  | "website"
  | "content"
  | "card_number"
  | "card_cvv"
  | "card_pin"
  | "cardholder"
  | "doc_number"
  | `custom:${number}`;

export interface FolderView {
  id: string;
  name: string;
  itemCount: number;
  createdAt: number;
}

export interface HelloStatus {
  /** Windows Hello is set up on this PC. */
  available: boolean;
  /** The active vault can be unlocked with Hello. */
  enabled: boolean;
}

export interface PasswordHealth {
  /** Logins with a password. */
  checked: number;
  /** Logins with no problem at all. */
  healthy: number;
  weak: { item: ItemSummary; level: number }[];
  /** Each group shares one password. */
  reused: ItemSummary[][];
  old: { item: ItemSummary; changedAt: number; ageDays: number }[];
  oldAfterDays: number;
}

export interface DashboardData {
  total: number;
  logins: number;
  notes: number;
  cards: number;
  identities: number;
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
  autoBackup: boolean;
  autoBackupIntervalHours: number;
  autoBackupKeep: number;
  /** `null` = the default folder inside the app data dir. */
  autoBackupDir: string | null;
}

export interface AutoBackupStatus {
  lastAt: number | null;
  folder: string;
  count: number;
}

export type ExportFormat = "csv" | "json";

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
