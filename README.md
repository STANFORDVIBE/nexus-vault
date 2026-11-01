# Nexus Vault

Локальный менеджер паролей и защищённых заметок для Windows. Без регистрации и облака:
всё хранится на компьютере в зашифрованном виде.

> Статус: в разработке (Phase 1 — Foundation). Хранилища ещё нет, пароли сюда пока не класть.

## Что нужно

- Node.js 20.19+
- Rust (stable, toolchain `x86_64-pc-windows-msvc`)
- Visual Studio Build Tools 2022 с компонентом «C++ build tools» и Windows SDK
- WebView2 (в Windows 10/11 уже есть)

## Запуск

```bash
npm install
npm run tauri dev
```

Установщик: `npm run tauri build` → `src-tauri/target/release/bundle/nsis/`.

## Проверки

```bash
npm run check
```

Типы, линтеры, тесты фронтенда и Rust. Перед `npm run check` должен быть собран
фронтенд (`npm run build`).

## Документация

- [PROJECT_SPEC.md](PROJECT_SPEC.md) — техническое задание
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — архитектура и решения по безопасности
