import { create } from "zustand";

interface UiState {
  /** Set by Ctrl+K / the tray shortcut; the item list focuses search and clears it. */
  searchFocusPending: boolean;
  requestSearchFocus: () => void;
  clearSearchFocus: () => void;
}

export const useUiStore = create<UiState>((set) => ({
  searchFocusPending: false,
  requestSearchFocus: () => set({ searchFocusPending: true }),
  clearSearchFocus: () => set({ searchFocusPending: false }),
}));
