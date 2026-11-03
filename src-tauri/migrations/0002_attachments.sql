-- Images attached to notes. The bytes (and their MIME type) are sealed with the Vault
-- Key like every item payload; the note text refers to them as `attachment:<id>`.
-- They are kept out of `items` so unlocking never has to decrypt them.

CREATE TABLE attachments (
    id             TEXT    PRIMARY KEY,
    item_id        TEXT    NOT NULL REFERENCES items (id) ON DELETE CASCADE,
    encrypted_data BLOB    NOT NULL,
    nonce          BLOB    NOT NULL,
    crypto_version INTEGER NOT NULL,
    created_at     INTEGER NOT NULL
);

CREATE INDEX idx_attachments_item ON attachments (item_id);
