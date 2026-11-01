import { create } from "zustand";
import * as vault from "@/services/vault";
import type { Settings } from "@/types/vault";

interface SettingsState {
  settings: Settings | null;
  load: () => Promise<void>;
  /** Saves a partial change; resolves to the stored settings (autostart may differ). */
  update: (patch: Partial<Settings>) => Promise<Settings>;
}

export const useSettingsStore = create<SettingsState>((set, get) => ({
  settings: null,
  load: async () => {
    set({ settings: await vault.getSettings() });
  },
  update: async (patch) => {
    const current = get().settings ?? (await vault.getSettings());
    const saved = await vault.updateSettings({ ...current, ...patch });
    set({ settings: saved });
    return saved;
  },
}));
