// In-memory stand-in for the Rust backend. Used only by tests, e2e and the browser
// preview (`npm run dev` outside Tauri). It is never bundled into a release build and
// implements behavior, not security: nothing here is encrypted.
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import type {
  CopyField,
  GeneratorOptions,
  ItemContent,
  ItemFilter,
  ItemInput,
  ItemSummary,
  Settings,
  VaultState,
} from "@/types/vault";

interface MockItem {
  id: string;
  content: ItemContent;
  folderId: string | null;
  favorite: boolean;
  createdAt: number;
  updatedAt: number;
  deletedAt: number | null;
}

interface MockFolder {
  id: string;
  name: string;
  createdAt: number;
}

export interface MockOptions {
  state?: VaultState;
  password?: string;
  seed?: boolean;
}

const fail = (code: string, message = code) => {
  throw { code, message };
};

function randomId(): string {
  return crypto.randomUUID();
}

function randomIndex(max: number): number {
  const buf = new Uint32Array(1);
  crypto.getRandomValues(buf);
  return (buf[0] ?? 0) % max;
}

function hostOf(website: string): string {
  return website
    .replace(/^[a-z]+:\/\//i, "")
    .replace(/^www\./, "")
    .split(/[/?#]/)[0]!;
}

export function createMockBackend(options: MockOptions = {}) {
  let state: VaultState = options.state ?? "uninitialized";
  let password = options.password ?? "correct horse battery";
  const items = new Map<string, MockItem>();
  const folders = new Map<string, MockFolder>();
  let settings: Settings = {
    autoLockMinutes: 5,
    clipboardClearSeconds: 30,
    lockOnSystemLock: true,
    closeBehavior: "lock_to_tray",
    launchAtStartup: false,
  };
  let pendingRestore = false;
  const clipboard: { value: string | null } = { value: null };

  const now = () => Date.now();

  function seed() {
    const work = randomId();
    folders.set(work, { id: work, name: "Работа", createdAt: now() });
    const add = (content: ItemContent, extra: Partial<MockItem> = {}) => {
      const id = randomId();
      const t = now() - randomIndex(10) * 3_600_000;
      items.set(id, {
        id,
        content,
        folderId: null,
        favorite: false,
        createdAt: t,
        updatedAt: t,
        deletedAt: null,
        ...extra,
      });
    };
    const login = (title: string, website: string, username: string, pw: string): ItemContent => ({
      type: "login",
      payload: { title, website, username, email: null, password: pw, notes: null, tags: [] },
    });
    add(login("GitHub", "https://github.com", "octocat", "gh-s3cret-pass"), { favorite: true });
    add(login("Почта", "https://mail.example.com", "me@example.com", "mail-pass-123"), {
      folderId: work,
    });
    add(login("Банк", "https://bank.example.ru", "client42", "bank-pass-9"));
    add({
      type: "secure_note",
      payload: {
        title: "Коды восстановления",
        content: "## GitHub\n\n- `1234-5678`\n- `8765-4321`\n\n[Справка](https://docs.github.com)",
        tags: ["2fa"],
      },
    });
  }
  if (options.seed) seed();

  function requireUnlocked() {
    if (state === "uninitialized") fail("vault_not_initialized");
    if (state !== "unlocked") fail("vault_locked");
  }

  function getItem(id: string): MockItem {
    const item = items.get(id);
    if (!item) return fail("not_found");
    return item;
  }

  function summary(item: MockItem): ItemSummary {
    const p = item.content.payload;
    const subtitle =
      item.content.type === "login"
        ? (item.content.payload.username ??
          item.content.payload.email ??
          (item.content.payload.website ? hostOf(item.content.payload.website) : null))
        : (item.content.payload.content
            .split("\n")
            .map((l) => l.trim().replace(/^[#>*\s-]+/, ""))
            .find(Boolean)
            ?.slice(0, 80) ?? null);
    return {
      id: item.id,
      type: item.content.type,
      title: p.title,
      subtitle,
      website: item.content.type === "login" ? (item.content.payload.website ?? null) : null,
      favorite: item.favorite,
      folderId: item.folderId,
      tags: p.tags,
      createdAt: item.createdAt,
      updatedAt: item.updatedAt,
      deletedAt: item.deletedAt,
    };
  }

  function validate(input: ItemInput) {
    if (!input.payload.title.trim()) fail("validation", "validation failed: title");
    if (input.folderId && !folders.has(input.folderId)) {
      fail("validation", "validation failed: folder");
    }
  }

  function listItems(filter: ItemFilter): ItemSummary[] {
    const terms = (filter.query ?? "").toLowerCase().split(/\s+/).filter(Boolean);
    const result = [...items.values()].filter((item) => {
      const trashed = item.deletedAt !== null;
      if (filter.view === "trash") {
        if (!trashed) return false;
      } else if (trashed) return false;
      if (filter.view === "logins" && item.content.type !== "login") return false;
      if (filter.view === "notes" && item.content.type !== "secure_note") return false;
      if (filter.view === "favorites" && !item.favorite) return false;
      if (filter.view === "folder" && item.folderId !== filter.folderId) return false;
      const haystack = JSON.stringify(item.content.payload).toLowerCase();
      return terms.every((t) => haystack.includes(t));
    });
    const sort = filter.sort ?? "updated";
    result.sort((a, b) =>
      sort === "title"
        ? a.content.payload.title.localeCompare(b.content.payload.title, "ru")
        : sort === "created"
          ? b.createdAt - a.createdAt
          : b.updatedAt - a.updatedAt,
    );
    return result.map(summary);
  }

  function detail(item: MockItem) {
    const base = {
      id: item.id,
      favorite: item.favorite,
      folderId: item.folderId,
      createdAt: item.createdAt,
      updatedAt: item.updatedAt,
      deletedAt: item.deletedAt,
    };
    if (item.content.type === "login") {
      const { password: pw, ...rest } = item.content.payload;
      return {
        ...base,
        type: "login",
        payload: {
          website: null,
          username: null,
          email: null,
          notes: null,
          ...rest,
          hasPassword: !!pw,
        },
      };
    }
    return { ...base, type: "secure_note", payload: item.content.payload };
  }

  function fieldValue(id: string, field: CopyField): string {
    const item = getItem(id);
    const value =
      item.content.type === "login"
        ? field === "content"
          ? null
          : item.content.payload[field]
        : field === "content"
          ? item.content.payload.content
          : null;
    if (!value) return fail("not_found");
    return value;
  }

  function generate(o: GeneratorOptions) {
    if (o.mode === "passphrase") {
      const words = ["лес", "river", "stone", "amber", "cloud", "north", "maple", "quartz"];
      const list = Array.from({ length: o.words }, () => {
        const w = words[randomIndex(words.length)]!;
        return o.capitalize ? w[0]!.toUpperCase() + w.slice(1) : w;
      });
      if (o.includeNumber) list.push(String(randomIndex(100)));
      return { value: list.join(o.separator), entropyBits: o.words * 11 };
    }
    let chars = "";
    if (o.uppercase) chars += "ABCDEFGHJKLMNPQRSTUVWXYZ";
    if (o.lowercase) chars += "abcdefghijkmnopqrstuvwxyz";
    if (o.digits) chars += "23456789";
    if (o.symbols) chars += "!@#$%^&*-_=+";
    if (!chars) fail("validation", "validation failed: charset");
    const value = Array.from({ length: o.length }, () => chars[randomIndex(chars.length)]).join("");
    return { value, entropyBits: Math.round(o.length * Math.log2(chars.length)) };
  }

  const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
    get_app_info: () => ({ name: "Nexus Vault", version: "0.1.0" }),
    get_vault_status: () => ({ state, corruptedItems: 0 }),
    create_vault: ({ password: pw }) => {
      if (state !== "uninitialized") fail("vault_already_exists");
      if ([...(pw as string)].length < 10) {
        fail("validation", "validation failed: master_password_too_short");
      }
      password = pw as string;
      state = "unlocked";
      return { state, corruptedItems: 0 };
    },
    unlock_vault: ({ password: pw }) => {
      if (state === "uninitialized") fail("vault_not_initialized");
      if (pw !== password) fail("invalid_password");
      state = "unlocked";
      return { state, corruptedItems: 0 };
    },
    lock_vault: () => {
      if (state === "unlocked") state = "locked";
      return { state, corruptedItems: 0 };
    },
    change_master_password: ({ current, new: next }) => {
      requireUnlocked();
      if (current !== password) fail("invalid_password");
      if ([...(next as string)].length < 10) {
        fail("validation", "validation failed: master_password_too_short");
      }
      password = next as string;
    },
    report_activity: () => undefined,
    list_items: ({ filter }) => {
      requireUnlocked();
      return listItems(filter as ItemFilter);
    },
    get_item: ({ id }) => {
      requireUnlocked();
      return detail(getItem(id as string));
    },
    get_dashboard: () => {
      requireUnlocked();
      const active = [...items.values()].filter((i) => i.deletedAt === null);
      const byUpdated = [...active].sort((a, b) => b.updatedAt - a.updatedAt);
      return {
        total: active.length,
        logins: active.filter((i) => i.content.type === "login").length,
        notes: active.filter((i) => i.content.type === "secure_note").length,
        favorites: active.filter((i) => i.favorite).length,
        trash: items.size - active.length,
        recent: byUpdated.slice(0, 6).map(summary),
        favoriteItems: byUpdated
          .filter((i) => i.favorite)
          .slice(0, 6)
          .map(summary),
      };
    },
    create_item: ({ input }) => {
      requireUnlocked();
      const i = input as ItemInput;
      validate(i);
      const id = randomId();
      const t = now();
      items.set(id, {
        id,
        content: { type: i.type, payload: i.payload } as ItemContent,
        folderId: i.folderId ?? null,
        favorite: i.favorite ?? false,
        createdAt: t,
        updatedAt: t,
        deletedAt: null,
      });
      return id;
    },
    update_item: ({ id, input }) => {
      requireUnlocked();
      const item = getItem(id as string);
      const i = input as ItemInput;
      if (i.type !== item.content.type) fail("validation", "validation failed: type");
      validate(i);
      item.content = { type: i.type, payload: i.payload } as ItemContent;
      item.folderId = i.folderId ?? null;
      item.updatedAt = now();
    },
    set_favorite: ({ id, favorite }) => {
      requireUnlocked();
      getItem(id as string).favorite = favorite as boolean;
    },
    move_item_to_folder: ({ id, folderId }) => {
      requireUnlocked();
      if (folderId && !folders.has(folderId as string)) {
        fail("validation", "validation failed: folder");
      }
      const item = getItem(id as string);
      item.folderId = (folderId as string | null) ?? null;
      item.updatedAt = now();
    },
    trash_item: ({ id }) => {
      requireUnlocked();
      getItem(id as string).deletedAt = now();
    },
    restore_item: ({ id }) => {
      requireUnlocked();
      getItem(id as string).deletedAt = null;
    },
    delete_item_permanently: ({ id }) => {
      requireUnlocked();
      if (getItem(id as string).deletedAt === null) {
        fail("validation", "validation failed: not_in_trash");
      }
      items.delete(id as string);
    },
    empty_trash: () => {
      requireUnlocked();
      let count = 0;
      for (const [id, item] of items) {
        if (item.deletedAt !== null) {
          items.delete(id);
          count++;
        }
      }
      return count;
    },
    reveal_password: ({ id }) => {
      requireUnlocked();
      return fieldValue(id as string, "password");
    },
    copy_item_field: ({ id, field }) => {
      requireUnlocked();
      clipboard.value = fieldValue(id as string, field as CopyField);
      return field === "password" || field === "content" ? settings.clipboardClearSeconds : 0;
    },
    open_item_website: ({ id }) => {
      requireUnlocked();
      fieldValue(id as string, "website");
    },
    open_external_url: ({ url }) => {
      if (!/^https?:\/\//i.test(url as string)) fail("validation", "validation failed: website");
    },
    list_folders: () => {
      requireUnlocked();
      return [...folders.values()]
        .sort((a, b) => a.name.localeCompare(b.name, "ru"))
        .map((f) => ({
          ...f,
          itemCount: [...items.values()].filter((i) => i.folderId === f.id && i.deletedAt === null)
            .length,
        }));
    },
    create_folder: ({ name }) => {
      requireUnlocked();
      const n = (name as string).trim();
      if (!n || n.length > 64) fail("validation", "validation failed: folder_name");
      if ([...folders.values()].some((f) => f.name.toLowerCase() === n.toLowerCase())) {
        fail("validation", "validation failed: folder_exists");
      }
      const id = randomId();
      folders.set(id, { id, name: n, createdAt: now() });
      return id;
    },
    rename_folder: ({ id, name }) => {
      requireUnlocked();
      const folder = folders.get(id as string) ?? fail("not_found");
      folder.name = (name as string).trim();
    },
    delete_folder: ({ id }) => {
      requireUnlocked();
      folders.delete(id as string);
      for (const item of items.values()) if (item.folderId === id) item.folderId = null;
    },
    generate_password: ({ options: o }) => generate(o as GeneratorOptions),
    copy_generated: ({ value }) => {
      clipboard.value = value as string;
      return settings.clipboardClearSeconds;
    },
    get_settings: () => settings,
    update_settings: ({ settings: next }) => {
      requireUnlocked();
      settings = next as Settings;
      return settings;
    },
    create_backup: () => {
      requireUnlocked();
      return "nexus-vault-mock.nexus";
    },
    pick_backup_file: () => {
      pendingRestore = true;
      return { fileName: "nexus-vault-mock.nexus", createdAt: now(), sizeBytes: 4096 };
    },
    restore_backup: ({ password: pw }) => {
      if (!pendingRestore) fail("cancelled");
      if (pw !== password) fail("invalid_password");
      pendingRestore = false;
      state = "locked";
      void emit("vault-locked");
      return { items: items.size, folders: folders.size };
    },
  };

  return {
    handle(cmd: string, args: Record<string, unknown> = {}) {
      const handler = handlers[cmd];
      if (!handler) throw new Error(`mock backend: unknown command ${cmd}`);
      return handler(args);
    },
    /** Simulates the Rust auto-lock / Win+L path. */
    lockFromBackend() {
      if (state === "unlocked") state = "locked";
      void emit("vault-locked");
    },
    clipboard,
    get state() {
      return state;
    },
  };
}

export type MockBackend = ReturnType<typeof createMockBackend>;

/** Routes every `invoke` and event to a fresh mock backend. */
export function installMockBackend(options?: MockOptions): MockBackend {
  const backend = createMockBackend(options);
  mockIPC(
    (cmd, payload) => {
      if (cmd.startsWith("plugin:")) return undefined;
      return backend.handle(cmd, payload as Record<string, unknown>);
    },
    { shouldMockEvents: true },
  );
  return backend;
}
