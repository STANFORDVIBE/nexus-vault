/** Error shape produced by `AppError` serialization in Rust. */
export interface AppError {
  code: string;
  message: string;
}

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof (value as AppError).code === "string"
  );
}

const VALIDATION_MESSAGES: Record<string, string> = {
  title: "Укажите название",
  website: "Слишком длинный или некорректный адрес сайта",
  username: "Слишком длинный логин",
  email: "Слишком длинный email",
  password: "Слишком длинный пароль",
  notes: "Слишком длинные заметки",
  content: "Заметка слишком большая",
  tags: "Не больше 32 тегов по 64 символа",
  folder: "Папка не найдена",
  folder_name: "Название папки: от 1 до 64 символов",
  folder_exists: "Папка с таким названием уже есть",
  master_password_too_short: "Мастер-пароль должен быть не короче 10 символов",
  master_password_too_long: "Мастер-пароль слишком длинный",
  length: "Длина пароля — от 8 до 128 символов",
  charset: "Выберите хотя бы один набор символов",
  words: "Количество слов — от 3 до 12",
  not_in_trash: "Сначала переместите запись в корзину",
  separator: "Разделитель — не больше 3 символов",
  auto_lock_minutes: "Автоблокировка — не больше 24 часов",
  clipboard_clear_seconds: "Очистка буфера — не дольше 10 минут",
  type: "Неизвестный тип записи",
  vault_name: "Название хранилища: от 1 до 64 символов",
  fields: "Не больше 50 своих полей; название — до 100 символов",
  cardholder: "Слишком длинное имя держателя",
  card_number: "Номер карты — от 12 до 19 цифр",
  cvv: "CVV — 3 или 4 цифры",
  pin: "PIN — от 4 до 12 цифр",
  expiry: "Срок действия в формате ММ/ГГ",
  kind: "Слишком длинный тип документа",
  full_name: "Слишком длинное ФИО",
  doc_number: "Слишком длинный номер документа",
  issued_by: "Слишком длинное поле «Кем выдан»",
  date: "Дата — не длиннее 32 символов",
  last_vault: "Нельзя убрать единственное хранилище",
  not_a_vault: "Этот файл не является хранилищем Nexus Vault",
  vault_unlocked: "Сначала заблокируйте хранилище",
  auto_backup_interval_hours: "Интервал автокопий — от 1 часа до 30 дней",
  auto_backup_keep: "Хранить можно от 1 до 100 автокопий",
  auto_backup_dir: "Выберите папку для автокопий заново",
  image: "Поддерживаются картинки PNG, JPEG, WebP и GIF",
  image_too_large: "Картинка слишком большая — не больше 20 МБ",
  too_many_images: "В одной заметке — не больше 200 картинок",
  item: "Сначала сохраните заметку",
};

const MESSAGES: Record<string, string> = {
  vault_not_initialized: "Хранилище ещё не создано",
  vault_already_exists: "Хранилище уже существует",
  vault_locked: "Хранилище заблокировано",
  invalid_password: "Неверный мастер-пароль",
  integrity: "Данные повреждены или изменены — проверка целостности не пройдена",
  not_found: "Запись не найдена",
  unsupported_version: "Файл создан более новой версией Nexus Vault",
  invalid_backup: "Файл повреждён или не является резервной копией Nexus Vault",
  cancelled: "Операция отменена",
  update_failed: "Не удалось получить обновление — проверьте интернет и попробуйте позже",
  hello_unavailable: "Windows Hello недоступен — войдите мастер-паролем",
  hello_failed:
    "Ключ Windows Hello больше не подходит — войдите мастер-паролем и включите Hello заново в настройках",
  database: "Ошибка базы данных",
  io: "Ошибка чтения или записи файла",
  internal: "Внутренняя ошибка",
};

/** Human-readable Russian message for any error thrown by an IPC call. */
export function errorMessage(error: unknown): string {
  if (isAppError(error)) {
    if (error.code === "validation") {
      const field = error.message.replace("validation failed: ", "");
      return VALIDATION_MESSAGES[field] ?? "Проверьте введённые данные";
    }
    return MESSAGES[error.code] ?? "Что-то пошло не так";
  }
  return "Что-то пошло не так";
}

export function hasCode(error: unknown, code: string): boolean {
  return isAppError(error) && error.code === code;
}
