# Nexus Vault

Локальный менеджер паролей и защищённых заметок для Windows. Без регистрации, облака
и интернета: всё хранится на компьютере в зашифрованном виде (Argon2id +
XChaCha20-Poly1305).

## Возможности

- Пароли и заметки в Markdown, папки, теги, избранное, корзина
- Мгновенный поиск (Ctrl+K), генератор паролей и парольных фраз
- Копирование в буфер с автоочисткой и без попадания в журнал Win+V
- Автоблокировка: по бездействию, при Win+L, при закрытии окна; работа из трея
- Глобальная клавиша Ctrl+Shift+L — открыть и сразу искать
- Зашифрованные резервные копии `.nexus` и восстановление из них
- Тёмная и светлая темы; интерфейс на русском

## Установка

Готовый установщик: `npm run tauri build` →
`src-tauri/target/release/bundle/nsis/Nexus Vault_0.1.0_x64-setup.exe`.
Ставится в профиль пользователя, права администратора не нужны.

## Разработка

Нужны Node.js 20.19+, Rust (stable, MSVC), Visual Studio Build Tools 2022 (C++ и Windows SDK).

```bash
npm install
npm run tauri dev
```

Проверки: `npm run check` (типы, линтеры, тесты фронтенда и Rust) и `npm run e2e`
(сценарии в Microsoft Edge). Интерфейс без Rust: `npm run dev` и открыть
`http://localhost:1420/?seed` — подключится фейковый бэкенд с тестовыми данными
(пароль `correct horse battery`).

## Документация

- [docs/SECURITY.md](docs/SECURITY.md) — что защищено и что нет
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — устройство и замеры производительности
- [PROJECT_SPEC.md](PROJECT_SPEC.md) — техническое задание
