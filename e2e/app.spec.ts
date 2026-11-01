import { expect, test, type Page } from "@playwright/test";

// `?seed` starts the mock backend locked, with sample items and this password
const PASSWORD = "correct horse battery";

async function unlock(page: Page) {
  await page.goto("/?seed");
  await page.getByPlaceholder("Мастер-пароль").fill(PASSWORD);
  await page.getByRole("button", { name: "Разблокировать" }).click();
  await expect(page.getByRole("heading", { name: "Обзор" })).toBeVisible();
}

test("first run: create a vault and land on an empty dashboard", async ({ page }) => {
  await page.goto("/");
  await page.getByLabel("Мастер-пароль").fill("e2e-master-password");
  await page.getByLabel("Повторите пароль").fill("e2e-master-password");
  await page.getByRole("checkbox").click();
  await page.getByRole("button", { name: "Создать хранилище" }).click();

  await expect(page.getByRole("heading", { name: "Обзор" })).toBeVisible();
  await expect(page.getByText("Хранилище пока пустое")).toBeVisible();
});

test("add a login with a generated password, then find it", async ({ page }) => {
  await unlock(page);
  await page.keyboard.press("Control+n");

  await page.getByLabel("Название").fill("Steam");
  await page.getByLabel("Логин").fill("gamer");
  await page.getByRole("button", { name: "Сгенерировать" }).click();
  await page.getByRole("button", { name: "Использовать" }).click();
  await expect(page.getByLabel("Пароль", { exact: true })).not.toHaveValue("");
  await page.keyboard.press("Control+s");

  await expect(page.getByText("Пароль сохранён")).toBeVisible();
  const list = page.getByRole("listbox", { name: "Записи" });
  await expect(list.getByText("Steam")).toBeVisible();

  await page.keyboard.press("Control+k");
  await page.keyboard.type("steam");
  await expect(list.getByRole("option")).toHaveCount(1);
});

test("write a markdown note with autosave", async ({ page }) => {
  await unlock(page);
  await page.getByRole("navigation").getByRole("link", { name: "Заметки" }).click();
  await page.keyboard.press("Control+Shift+n");

  await page.getByLabel("Название").fill("Wi-Fi");
  await page.getByLabel("Текст заметки").fill("## Дом\n\n- сеть: `home`\n");
  await expect(page.getByText("Сохранено")).toBeVisible();

  await page.getByRole("button", { name: "Готово" }).click();
  await expect(page.locator(".markdown h2")).toHaveText("Дом");
  await expect(page.locator(".markdown code")).toHaveText("home");
});

test("trash, restore and empty the trash", async ({ page }) => {
  await unlock(page);
  await page.getByRole("link", { name: "Все записи" }).click();
  const list = page.getByRole("listbox", { name: "Записи" });

  await list.getByText("Банк").click();
  await page.getByRole("button", { name: "Ещё" }).click();
  await page.getByRole("menuitem", { name: "В корзину" }).click();
  await expect(list.getByText("Банк")).toHaveCount(0);

  await page.getByRole("link", { name: "Корзина" }).click();
  await list.getByText("Банк").click();
  await page.getByRole("button", { name: "Восстановить" }).click();
  await expect(list.getByText("Банк")).toHaveCount(0);

  await page.getByRole("link", { name: "Все записи" }).click();
  await list.getByText("Почта").click();
  await page.getByRole("button", { name: "Ещё" }).click();
  await page.getByRole("menuitem", { name: "В корзину" }).click();
  await page.getByRole("link", { name: "Корзина" }).click();
  await page.getByRole("button", { name: "Очистить корзину" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Очистить" }).click();
  await expect(page.getByText("Корзина пуста")).toBeVisible();
});

test("folders: create, file an item, delete keeps the item", async ({ page }) => {
  await unlock(page);
  await page.getByRole("navigation").getByRole("link", { name: "Папки", exact: true }).click();
  await page.getByRole("button", { name: "Новая папка" }).click();
  await page.getByLabel("Название").fill("Игры");
  await page.getByRole("button", { name: "Создать" }).click();
  await expect(page.getByRole("link", { name: /Игры/ }).first()).toBeVisible();

  await page.getByRole("link", { name: "Все записи" }).click();
  await page.getByRole("option", { name: /^GitHub/ }).click();
  await page.getByRole("button", { name: "Ещё" }).click();
  await page.getByRole("menuitem", { name: "Переместить в папку" }).click();
  await page.getByRole("menuitemradio", { name: "Игры" }).click();
  await expect(page.getByText("Запись перемещена")).toBeVisible();

  await page.getByRole("navigation").getByRole("link", { name: /Игры/ }).click();
  await expect(page.getByRole("option", { name: /^GitHub/ })).toBeVisible();

  await page.getByRole("navigation").getByRole("link", { name: "Папки", exact: true }).click();
  await page.getByRole("button", { name: "Действия с папкой Игры" }).click();
  await page.getByRole("menuitem", { name: "Удалить" }).click();
  await page.getByRole("alertdialog").getByRole("button", { name: "Удалить папку" }).click();
  await expect(page.getByRole("link", { name: /Игры/ })).toHaveCount(0);

  await page.getByRole("link", { name: "Все записи" }).click();
  await expect(page.getByRole("option", { name: /^GitHub/ })).toBeVisible();
});

test("lock with Ctrl+L and unlock again", async ({ page }) => {
  await unlock(page);
  await page.keyboard.press("Control+l");
  await expect(page.getByRole("heading", { name: "Хранилище заблокировано" })).toBeVisible();

  await page.getByPlaceholder("Мастер-пароль").fill("wrong");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("alert")).toHaveText("Неверный мастер-пароль");

  await page.getByPlaceholder("Мастер-пароль").fill(PASSWORD);
  await page.keyboard.press("Enter");
  await expect(page.getByText("Разблокировано")).toBeVisible();
});

test("settings: theme switch persists", async ({ page }) => {
  await unlock(page);
  await page.getByRole("link", { name: "Настройки" }).click();
  await page.getByRole("radio", { name: "Светлая" }).click();
  await expect(page.locator("html")).not.toHaveClass(/dark/);
  await page.getByRole("radio", { name: "Тёмная" }).click();
  await expect(page.locator("html")).toHaveClass(/dark/);
});
