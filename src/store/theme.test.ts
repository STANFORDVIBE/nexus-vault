import { initTheme, resolveTheme, useThemeStore } from "@/store/theme";

vi.mock("@/services/window", () => ({ setWindowTheme: vi.fn() }));

describe("theme store", () => {
  beforeEach(() => {
    localStorage.clear();
    useThemeStore.setState({ preference: "dark", resolved: "dark" });
  });

  it("resolves 'system' from the OS preference", () => {
    expect(resolveTheme("system")).toBe("dark");
    expect(resolveTheme("light")).toBe("light");
  });

  it("persists the preference and toggles the dark class", () => {
    initTheme();
    expect(document.documentElement).toHaveClass("dark");

    useThemeStore.getState().setPreference("light");

    expect(localStorage.getItem("nexus-vault:theme")).toBe("light");
    expect(useThemeStore.getState().resolved).toBe("light");
    expect(document.documentElement).not.toHaveClass("dark");
  });

  it("keeps working when storage throws", () => {
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("denied");
    });

    expect(() => useThemeStore.getState().setPreference("system")).not.toThrow();
    expect(useThemeStore.getState().preference).toBe("system");
  });
});
