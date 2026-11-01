-- Nexus Vault schema v1.
-- Everything sensitive (titles, URLs, usernames, passwords, notes, tags, folder names)
-- is stored only inside XChaCha20-Poly1305 ciphertexts. Plaintext columns are limited
-- to random identifiers, item kind, flags and timestamps.

CREATE TABLE vault_meta (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    vault_id        TEXT    NOT NULL,
    format_version  INTEGER NOT NULL,
    kdf_algorithm   TEXT    NOT NULL,
    kdf_memory_kib  INTEGER NOT NULL,
    kdf_iterations  INTEGER NOT NULL,
    kdf_parallelism INTEGER NOT NULL,
    kdf_salt        BLOB    NOT NULL,
    created_at      INTEGER NOT NULL
);

-- Vault Key wrapped by the KEK derived from the master password.
CREATE TABLE key_envelopes (
    id             TEXT    PRIMARY KEY,
    wrapped_key    BLOB    NOT NULL,
    nonce          BLOB    NOT NULL,
    crypto_version INTEGER NOT NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
);

CREATE TABLE folders (
    id             TEXT    PRIMARY KEY,
    encrypted_name BLOB    NOT NULL,
    nonce          BLOB    NOT NULL,
    crypto_version INTEGER NOT NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
);

CREATE TABLE items (
    id                TEXT    PRIMARY KEY,
    item_type         TEXT    NOT NULL,
    encrypted_payload BLOB    NOT NULL,
    nonce             BLOB    NOT NULL,
    crypto_version    INTEGER NOT NULL,
    folder_id         TEXT    REFERENCES folders (id) ON DELETE SET NULL,
    is_favorite       INTEGER NOT NULL DEFAULT 0,
    created_at        INTEGER NOT NULL,
    updated_at        INTEGER NOT NULL,
    deleted_at        INTEGER
);

CREATE INDEX idx_items_folder ON items (folder_id);
CREATE INDEX idx_items_deleted ON items (deleted_at);

-- Non-secret application preferences (auto-lock timeout, close behaviour, ...).
CREATE TABLE settings (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
