import { create } from "zustand";
import * as vault from "@/services/vault";
import type { VaultStatus } from "@/types/vault";

interface SessionState {
  /** `null` until the first status check finishes. */
  status: VaultStatus | null;
  /** Bumped after every mutation; data hooks refetch when it changes. */
  revision: number;
  refresh: () => Promise<void>;
  setStatus: (status: VaultStatus) => void;
  /** Called on the Rust "vault-locked" event and after a manual lock. */
  markLocked: () => void;
  lock: () => Promise<void>;
  invalidate: () => void;
}

export const useSessionStore = create<SessionState>((set) => ({
  status: null,
  revision: 0,
  refresh: async () => {
    const status = await vault.getVaultStatus();
    set({ status });
  },
  setStatus: (status) => set((s) => ({ status, revision: s.revision + 1 })),
  markLocked: () =>
    set((s) =>
      s.status?.state === "unlocked"
        ? { status: { ...s.status, state: "locked", corruptedItems: 0 } }
        : s,
    ),
  lock: async () => {
    const status = await vault.lockVault();
    set({ status });
  },
  invalidate: () => set((s) => ({ revision: s.revision + 1 })),
}));

/** Shorthand for components that only need to trigger a refetch. */
export const invalidate = () => useSessionStore.getState().invalidate();
