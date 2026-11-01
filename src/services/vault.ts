// Typed wrappers over the Rust IPC commands. Components never call `invoke` directly.
import { invoke } from "@tauri-apps/api/core";
import type {
  BackupInfo,
  CopyField,
  DashboardData,
  FolderView,
  GeneratedPassword,
  GeneratorOptions,
  ItemDetail,
  ItemContent,
  ItemFilter,
  ItemInput,
  ItemSummary,
  AutoBackupStatus,
  ExportFormat,
  RestoreSummary,
  Settings,
  VaultListEntry,
  VaultStatus,
} from "@/types/vault";

// ---- vault lifecycle ----
export const getVaultStatus = () => invoke<VaultStatus>("get_vault_status");
export const createVault = (password: string) => invoke<VaultStatus>("create_vault", { password });
export const unlockVault = (password: string) => invoke<VaultStatus>("unlock_vault", { password });
export const lockVault = () => invoke<VaultStatus>("lock_vault");
export const changeMasterPassword = (current: string, next: string) =>
  invoke<void>("change_master_password", { current, new: next });
export const reportActivity = () => invoke<void>("report_activity");

// ---- several vaults ----
export const listVaults = () => invoke<VaultListEntry[]>("list_vaults");
export const switchVault = (id: string) => invoke<VaultStatus>("switch_vault", { id });
/** Resolves to `null` when the folder dialog was cancelled. */
export const addNewVault = (name: string, chooseFolder: boolean) =>
  invoke<VaultStatus | null>("add_new_vault", { name, chooseFolder });
export const openVaultFile = () => invoke<VaultStatus | null>("open_vault_file");
export const renameVault = (id: string, name: string) =>
  invoke<VaultStatus>("rename_vault", { id, name });
export const forgetVault = (id: string) => invoke<VaultStatus>("forget_vault", { id });
export const revealVault = (id: string) => invoke<void>("reveal_vault", { id });
/** Resolves to the file name the forgotten vault was renamed to. */
export const resetForgottenVault = () => invoke<string>("reset_forgotten_vault");

// ---- items ----
export const listItems = (filter: ItemFilter) => invoke<ItemSummary[]>("list_items", { filter });
export const getItem = (id: string) => invoke<ItemDetail>("get_item", { id });
export const getDashboard = () => invoke<DashboardData>("get_dashboard");
export const createItem = (input: ItemInput) => invoke<string>("create_item", { input });
export const updateItem = (id: string, input: ItemInput) =>
  invoke<void>("update_item", { id, input });
export const setFavorite = (id: string, favorite: boolean) =>
  invoke<void>("set_favorite", { id, favorite });
export const moveItemToFolder = (id: string, folderId: string | null) =>
  invoke<void>("move_item_to_folder", { id, folderId });
export const trashItem = (id: string) => invoke<void>("trash_item", { id });
export const restoreItem = (id: string) => invoke<void>("restore_item", { id });
export const deleteItemPermanently = (id: string) =>
  invoke<void>("delete_item_permanently", { id });
export const emptyTrash = () => invoke<number>("empty_trash");
export const revealPassword = (id: string) => invoke<string>("reveal_password", { id });
export const revealField = (id: string, field: CopyField) =>
  invoke<string>("reveal_field", { id, field });
/** Full content with secrets, only to fill an edit form. */
export const getItemForEdit = (id: string) => invoke<ItemContent>("get_item_for_edit", { id });
/** Copies in Rust; resolves to the auto-clear timeout in seconds (0 = not cleared). */
export const copyItemField = (id: string, field: CopyField) =>
  invoke<number>("copy_item_field", { id, field });
export const openItemWebsite = (id: string) => invoke<void>("open_item_website", { id });
export const openExternalUrl = (url: string) => invoke<void>("open_external_url", { url });

// ---- folders ----
export const listFolders = () => invoke<FolderView[]>("list_folders");
export const createFolder = (name: string) => invoke<string>("create_folder", { name });
export const renameFolder = (id: string, name: string) =>
  invoke<void>("rename_folder", { id, name });
export const deleteFolder = (id: string) => invoke<void>("delete_folder", { id });

// ---- generator ----
export const generatePassword = (options: GeneratorOptions) =>
  invoke<GeneratedPassword>("generate_password", { options });
export const copyGenerated = (value: string) => invoke<number>("copy_generated", { value });

// ---- settings ----
export const getSettings = () => invoke<Settings>("get_settings");
export const updateSettings = (settings: Settings) =>
  invoke<Settings>("update_settings", { settings });

// ---- backup ----
export const createBackup = () => invoke<string | null>("create_backup");
export const pickBackupFile = () => invoke<BackupInfo | null>("pick_backup_file");
export const restoreBackup = (password: string) =>
  invoke<RestoreSummary>("restore_backup", { password });
export const getAutoBackupStatus = () => invoke<AutoBackupStatus>("get_auto_backup_status");
/** Resolves to the file name of the new copy. */
export const runAutoBackupNow = () => invoke<string>("run_auto_backup_now");
export const chooseAutoBackupFolder = () => invoke<string | null>("choose_auto_backup_folder");
export const openAutoBackupFolder = () => invoke<void>("open_auto_backup_folder");
/** Unencrypted export; resolves to the file name, or `null` if the dialog was cancelled. */
export const exportVault = (password: string, format: ExportFormat) =>
  invoke<string | null>("export_vault", { password, format });
