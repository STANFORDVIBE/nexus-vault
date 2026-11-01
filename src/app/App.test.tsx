import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { App } from "@/app/App";
import { getAppInfo } from "@/services/app";

vi.mock("@/services/app", () => ({ getAppInfo: vi.fn() }));
vi.mock("@/services/window", () => ({ setWindowTheme: vi.fn() }));

describe("App shell", () => {
  beforeEach(() => {
    window.history.pushState({}, "", "/");
    vi.mocked(getAppInfo).mockResolvedValue({ name: "Nexus Vault", version: "0.1.0" });
  });

  it("opens the design system and shows every sidebar section", async () => {
    render(<App />);

    expect(await screen.findByRole("heading", { name: "Дизайн-система" })).toBeInTheDocument();

    const nav = screen.getByRole("navigation", { name: "Основная навигация" });
    for (const label of [
      "Обзор",
      "Все записи",
      "Пароли",
      "Заметки",
      "Избранное",
      "Папки",
      "Корзина",
      "Настройки",
    ]) {
      expect(within(nav).getByRole("link", { name: label })).toBeInTheDocument();
    }
  });

  it("shows the backend version and an honest vault status", async () => {
    render(<App />);

    expect(await screen.findByText("v0.1.0")).toBeInTheDocument();
    expect(screen.getByText("Хранилище не создано")).toBeInTheDocument();
  });

  it("navigates between sections and marks the active one", async () => {
    const user = userEvent.setup();
    render(<App />);

    await user.click(screen.getByRole("link", { name: "Пароли" }));

    expect(screen.getByText("Пароли: ещё в разработке")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Пароли" })).toHaveAttribute("aria-current", "page");
  });
});
