import { act, cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { clearMocks } from "@tauri-apps/api/mocks";
import { App } from "@/app/App";
import { installMockBackend, type MockOptions } from "@/dev/mockBackend";
import { useSessionStore } from "@/store/session";

vi.mock("@/services/window", () => ({ setWindowTheme: vi.fn() }));

const PASSWORD = "correct horse battery";

function setup(options?: MockOptions) {
  const backend = installMockBackend(options);
  useSessionStore.setState({ status: null, revision: 0 });
  const user = userEvent.setup();
  render(<App />);
  return { backend, user };
}

async function unlock(user: ReturnType<typeof userEvent.setup>) {
  await user.type(await screen.findByPlaceholderText("Мастер-пароль"), PASSWORD);
  await user.click(screen.getByRole("button", { name: /Разблокировать/ }));
  await screen.findByRole("navigation", { name: "Основная навигация" });
}

describe("Nexus Vault app", () => {
  beforeEach(() => window.history.pushState({}, "", "/"));
  afterEach(async () => {
    // unmount and let the async unlisten calls finish while the mocked IPC still exists
    cleanup();
    await new Promise((resolve) => setTimeout(resolve, 0));
    clearMocks();
  });

  it("creates a vault on first run and opens the dashboard", async () => {
    const { user } = setup();

    expect(await screen.findByRole("heading", { name: "Создайте хранилище" })).toBeInTheDocument();
    const create = screen.getByRole("button", { name: "Создать хранилище" });

    await user.type(screen.getByLabelText("Мастер-пароль"), "short");
    await user.type(screen.getByLabelText("Повторите пароль"), "short");
    await user.click(screen.getByRole("checkbox"));
    expect(create).toBeDisabled(); // under 10 characters

    await user.type(screen.getByLabelText("Мастер-пароль"), "-but-long-enough");
    await user.type(screen.getByLabelText("Повторите пароль"), "-but-different");
    expect(screen.getByText("Пароли не совпадают")).toBeInTheDocument();
    expect(create).toBeDisabled();

    await user.clear(screen.getByLabelText("Повторите пароль"));
    await user.type(screen.getByLabelText("Повторите пароль"), "short-but-long-enough");
    await user.click(create);

    expect(await screen.findByRole("heading", { name: "Обзор" })).toBeInTheDocument();
    expect(await screen.findByText("Хранилище пока пустое")).toBeInTheDocument();
    expect(screen.getByText("Разблокировано")).toBeInTheDocument();
  });

  it("rejects a wrong master password and unlocks with the right one", async () => {
    const { user } = setup({ state: "locked", seed: true });

    await user.type(await screen.findByPlaceholderText("Мастер-пароль"), "nope");
    await user.click(screen.getByRole("button", { name: /Разблокировать/ }));
    expect(await screen.findByRole("alert")).toHaveTextContent("Неверный мастер-пароль");

    await user.clear(screen.getByPlaceholderText("Мастер-пароль"));
    await unlock(user);
    expect((await screen.findAllByText("GitHub")).length).toBeGreaterThan(0);
  });

  it("returns to the lock screen when the backend locks the vault", async () => {
    const { backend, user } = setup({ state: "locked", seed: true });
    await unlock(user);

    act(() => backend.lockFromBackend());

    expect(
      await screen.findByRole("heading", { name: "Хранилище заблокировано" }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Основная навигация" })).toBeNull();
  });

  it("adds a login, then finds it with search", async () => {
    window.history.pushState({}, "", "/passwords");
    const { backend, user } = setup({ state: "locked", seed: true });
    await unlock(user);

    await user.click(await screen.findByRole("button", { name: "Добавить" }));
    await user.type(await screen.findByLabelText("Название"), "Steam");
    await user.type(screen.getByLabelText("Логин"), "gamer");
    await user.type(screen.getByLabelText("Пароль"), "pa55-word");
    await user.click(screen.getByRole("button", { name: "Сохранить" }));

    const list = await screen.findByRole("listbox", { name: "Записи" });
    expect(await within(list).findByText("Steam")).toBeInTheDocument();
    // the detail view never receives the password itself
    expect(screen.queryByText("pa55-word")).toBeNull();

    await user.type(screen.getByRole("searchbox", { name: "Поиск" }), "steam");
    await waitFor(() => expect(within(list).getAllByRole("option")).toHaveLength(1));

    await user.click(screen.getAllByRole("button", { name: "Копировать" })[1]!);
    expect(backend.clipboard.value).toBe("pa55-word");
  });

  it("moves an item to the trash and restores it", async () => {
    window.history.pushState({}, "", "/items");
    const { user } = setup({ state: "locked", seed: true });
    await unlock(user);

    await user.click(await screen.findByText("Банк"));
    await user.click(await screen.findByRole("button", { name: "Ещё" }));
    await user.click(await screen.findByRole("menuitem", { name: "В корзину" }));
    await waitFor(() => expect(screen.queryByText("Банк")).toBeNull());

    await user.click(screen.getByRole("link", { name: "Корзина" }));
    await user.click(await screen.findByText("Банк"));
    await user.click(await screen.findByRole("button", { name: "Восстановить" }));
    await waitFor(() => expect(screen.queryByText("Банк")).toBeNull());

    await user.click(screen.getByRole("link", { name: "Все записи" }));
    expect(await screen.findByText("Банк")).toBeInTheDocument();
  });

  it("creates a second vault and switches back to the first one", async () => {
    const { user } = setup({ state: "locked", seed: true });
    await unlock(user);

    await user.click(screen.getByRole("button", { name: /Сменить хранилище/ }));
    await user.click(await screen.findByRole("menuitem", { name: /Создать новое хранилище/ }));
    await user.type(screen.getByLabelText("Название"), "Работа");
    await user.click(screen.getByRole("button", { name: "Продолжить" }));

    expect(
      await screen.findByRole("heading", { name: "Создайте хранилище «Работа»" }),
    ).toBeInTheDocument();
    await user.type(screen.getByLabelText("Мастер-пароль"), "work-master-password");
    await user.type(screen.getByLabelText("Повторите пароль"), "work-master-password");
    await user.click(screen.getByRole("checkbox"));
    await user.click(screen.getByRole("button", { name: "Создать хранилище" }));
    expect(await screen.findByText("Хранилище пока пустое")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Хранилище: Работа/ }));
    await user.click(await screen.findByRole("menuitem", { name: "Основное" }));
    expect(await screen.findByText("«Основное»")).toBeInTheDocument();
    await unlock(user);
    expect((await screen.findAllByText("GitHub")).length).toBeGreaterThan(0);
  });

  it("forgot password: starts over only after explicit confirmation", async () => {
    const { user } = setup({ state: "locked", seed: true });
    await user.click(await screen.findByRole("button", { name: "Забыли пароль?" }));

    const startOver = screen.getByRole("button", { name: "Начать заново" });
    expect(startOver).toBeDisabled();
    await user.click(screen.getByRole("checkbox"));
    await user.click(startOver);

    expect(await screen.findByRole("heading", { name: "Создайте хранилище" })).toBeInTheDocument();
    expect(await screen.findByText(/Старый файл не удалён/)).toBeInTheDocument();
  });
});
