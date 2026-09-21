-- SQLite — `contacts` is an ATTACHed database file, attached on every pooled
-- connection by kubuno-db, so the qualified names below resolve as they do on
-- the other two engines. This single file declares the FINAL shape the
-- PostgreSQL side reached across its 000001..000010 migrations.
--
-- Differences from PostgreSQL, and why:
--   * UUID -> BLOB, TIMESTAMPTZ -> TEXT (`%F %T%.f`, UTC), as sqlx encodes them.
--   * No DEFAULT on `id`: SQLite has no UUID generator; the process supplies it.
--   * JSONB -> TEXT holding JSON (the contact field arrays, settings/criteria).
--   * Full-text search is the normalized-column form (name_norm / org_norm /
--     email_norm / phone_norm / job_norm / notes_norm / addr_norm, filled in
--     Rust): no tsvector, no unaccent. display_name and etag are computed in
--     Rust (their PostgreSQL triggers are gone).
--   * The delta layer is the journal (change_counter + per-row change_seq) plus
--     the four tombstone tables; no sequences, no triggers. updated_at is
--     maintained in Rust.
--   * Foreign-key REFERENCES are unqualified (SQLite assumes the same database);
--     kubuno-db enables `PRAGMA foreign_keys`, so CASCADE deletes fire.

CREATE TABLE contacts.contacts (
    id               BLOB    NOT NULL PRIMARY KEY,
    owner_id         BLOB    NOT NULL,
    given_name       TEXT,
    middle_name      TEXT,
    family_name      TEXT,
    name_prefix      TEXT,
    name_suffix      TEXT,
    nickname         TEXT,
    display_name     TEXT    NOT NULL DEFAULT '',
    organization     TEXT,
    department       TEXT,
    job_title        TEXT,
    avatar_path      TEXT,
    avatar_color     TEXT    NOT NULL DEFAULT '#1a73e8',
    emails           TEXT    NOT NULL,
    phones           TEXT    NOT NULL,
    addresses        TEXT    NOT NULL,
    urls             TEXT    NOT NULL,
    dates            TEXT    NOT NULL,
    relations        TEXT    NOT NULL,
    instant_messages TEXT    NOT NULL,
    custom_fields    TEXT    NOT NULL,
    notes            TEXT,
    is_starred       INTEGER NOT NULL DEFAULT 0,
    is_trashed       INTEGER NOT NULL DEFAULT 0,
    trashed_at       TEXT,
    kubuno_user_id   BLOB,
    is_archived      INTEGER NOT NULL DEFAULT 0,
    archived_at      TEXT,
    is_blocked       INTEGER NOT NULL DEFAULT 0,
    last_interaction_at TEXT,
    interaction_count   INTEGER NOT NULL DEFAULT 0,
    pronouns         TEXT,
    vcard_uid        TEXT    NOT NULL UNIQUE,
    etag             TEXT    NOT NULL DEFAULT '',
    import_source    TEXT    NOT NULL DEFAULT 'manual',
    name_norm        TEXT    NOT NULL DEFAULT '',
    org_norm         TEXT    NOT NULL DEFAULT '',
    email_norm       TEXT    NOT NULL DEFAULT '',
    phone_norm       TEXT    NOT NULL DEFAULT '',
    job_norm         TEXT    NOT NULL DEFAULT '',
    notes_norm       TEXT    NOT NULL DEFAULT '',
    addr_norm        TEXT    NOT NULL DEFAULT '',
    change_seq       INTEGER NOT NULL DEFAULT 0,
    created_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at       TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX contacts.idx_contacts_owner       ON contacts(owner_id);
CREATE INDEX contacts.idx_contacts_trashed     ON contacts(owner_id, is_trashed);
CREATE INDEX contacts.idx_contacts_display     ON contacts(owner_id, display_name);
CREATE INDEX contacts.idx_contacts_vcard_uid   ON contacts(vcard_uid);
CREATE INDEX contacts.idx_contacts_kubuno_user ON contacts(kubuno_user_id);
CREATE INDEX contacts.idx_contacts_change_seq  ON contacts(owner_id, change_seq);
CREATE INDEX contacts.idx_contacts_last_interaction ON contacts(owner_id, last_interaction_at);
CREATE INDEX contacts.idx_contacts_name_norm   ON contacts(name_norm);
CREATE INDEX contacts.idx_contacts_org_norm    ON contacts(org_norm);

CREATE TABLE contacts.interaction_log (
    id               BLOB    NOT NULL PRIMARY KEY,
    contact_id       BLOB    NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    owner_id         BLOB    NOT NULL,
    interaction_type TEXT    NOT NULL,
    summary          TEXT,
    source_module    TEXT,
    source_id        BLOB,
    occurred_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX contacts.idx_contacts_interactions ON interaction_log(contact_id, occurred_at);

CREATE TABLE contacts.groups (
    id          BLOB    NOT NULL PRIMARY KEY,
    owner_id    BLOB    NOT NULL,
    name        TEXT    NOT NULL,
    color       TEXT    NOT NULL DEFAULT '#1a73e8',
    is_system   INTEGER NOT NULL DEFAULT 0,
    change_seq  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (owner_id, name)
);
CREATE INDEX contacts.idx_contacts_groups_owner     ON groups(owner_id);
CREATE INDEX contacts.idx_contacts_group_change_seq ON groups(owner_id, change_seq);

CREATE TABLE contacts.group_members (
    group_id    BLOB NOT NULL REFERENCES groups(id)   ON DELETE CASCADE,
    contact_id  BLOB NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    added_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (group_id, contact_id)
);
CREATE INDEX contacts.idx_contacts_gm_contact ON group_members(contact_id);

CREATE TABLE contacts.labels (
    id          BLOB    NOT NULL PRIMARY KEY,
    owner_id    BLOB    NOT NULL,
    name        TEXT    NOT NULL,
    color       TEXT    NOT NULL DEFAULT '#5f6368',
    icon        TEXT,
    is_system   INTEGER NOT NULL DEFAULT 0,
    position    INTEGER NOT NULL DEFAULT 0,
    change_seq  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (owner_id, name)
);
CREATE INDEX contacts.idx_contacts_labels_owner     ON labels(owner_id, position);
CREATE INDEX contacts.idx_contacts_label_change_seq ON labels(owner_id, change_seq);

CREATE TABLE contacts.contact_labels (
    label_id    BLOB NOT NULL REFERENCES labels(id)   ON DELETE CASCADE,
    contact_id  BLOB NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    added_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (label_id, contact_id)
);
CREATE INDEX contacts.idx_contacts_cl_contact ON contact_labels(contact_id);

CREATE TABLE contacts.saved_filters (
    id          BLOB    NOT NULL PRIMARY KEY,
    owner_id    BLOB    NOT NULL,
    name        TEXT    NOT NULL,
    icon        TEXT    NOT NULL DEFAULT 'Filter',
    color       TEXT    NOT NULL DEFAULT '#1a73e8',
    criteria    TEXT    NOT NULL,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    UNIQUE (owner_id, name)
);
CREATE INDEX contacts.idx_contacts_filters_owner ON saved_filters(owner_id, position);

CREATE TABLE contacts.reminders (
    id          BLOB    NOT NULL PRIMARY KEY,
    owner_id    BLOB    NOT NULL,
    contact_id  BLOB    NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    kind        TEXT    NOT NULL DEFAULT 'follow_up',
    message     TEXT,
    remind_at   TEXT    NOT NULL,
    recurrence  TEXT    NOT NULL DEFAULT 'none',
    is_done     INTEGER NOT NULL DEFAULT 0,
    notified_at TEXT,
    change_seq  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX contacts.idx_contacts_reminders_due       ON reminders(remind_at);
CREATE INDEX contacts.idx_contacts_reminders_owner     ON reminders(owner_id, remind_at);
CREATE INDEX contacts.idx_contacts_reminder_change_seq ON reminders(owner_id, change_seq);

CREATE TABLE contacts.shares (
    id            BLOB    NOT NULL PRIMARY KEY,
    owner_id      BLOB    NOT NULL,
    contact_id    BLOB    REFERENCES contacts(id) ON DELETE CASCADE,
    group_id      BLOB    REFERENCES groups(id)   ON DELETE CASCADE,
    token         TEXT    NOT NULL UNIQUE,
    permission    TEXT    NOT NULL DEFAULT 'view',
    expires_at    TEXT,
    password_hash TEXT,
    max_accesses  INTEGER,
    access_count  INTEGER NOT NULL DEFAULT 0,
    created_at    TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX contacts.idx_contacts_shares_owner ON shares(owner_id);
CREATE INDEX contacts.idx_contacts_shares_token ON shares(token);

CREATE TABLE contacts.change_log (
    id          INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    contact_id  BLOB    NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    owner_id    BLOB    NOT NULL,
    field       TEXT    NOT NULL,
    old_value   TEXT,
    new_value   TEXT,
    changed_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX contacts.idx_contacts_changelog ON change_log(contact_id, changed_at);

CREATE TABLE contacts.dedup_ignored (
    owner_id    BLOB NOT NULL,
    contact_a   BLOB NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    contact_b   BLOB NOT NULL REFERENCES contacts(id) ON DELETE CASCADE,
    created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    PRIMARY KEY (owner_id, contact_a, contact_b)
);

CREATE TABLE contacts.user_settings (
    owner_id    BLOB NOT NULL PRIMARY KEY,
    prefs       TEXT NOT NULL,
    updated_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);

CREATE TABLE contacts.carddav_tokens (
    owner_id     BLOB NOT NULL PRIMARY KEY,
    token_hash   TEXT NOT NULL,
    created_at   TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    last_used_at TEXT
);
CREATE INDEX contacts.idx_contacts_carddav_hash ON carddav_tokens(token_hash);

CREATE TABLE contacts.shared_contacts (
    id            BLOB NOT NULL PRIMARY KEY,
    display_name  TEXT NOT NULL,
    organization  TEXT,
    job_title     TEXT,
    email         TEXT,
    phone         TEXT,
    notes         TEXT,
    updated_by    BLOB,
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now'))
);
CREATE INDEX contacts.idx_shared_contacts_name ON shared_contacts(display_name);

CREATE TABLE contacts.other_contacts (
    id            BLOB    NOT NULL PRIMARY KEY,
    owner_id      BLOB    NOT NULL,
    kind          TEXT    NOT NULL,
    value         TEXT    NOT NULL,
    display_name  TEXT,
    source_module TEXT    NOT NULL,
    first_seen_at TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    last_seen_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%d %H:%M:%f', 'now')),
    seen_count    INTEGER NOT NULL DEFAULT 1 CHECK (seen_count > 0),
    dismissed_at  TEXT,
    CONSTRAINT contacts_other_unique UNIQUE (owner_id, kind, value)
);
CREATE INDEX contacts.idx_contacts_other_owner ON other_contacts(owner_id, last_seen_at);

-- ── Delta journal (portable change layer) ────────────────────────────────────
CREATE TABLE contacts.change_counter (
    domain TEXT   NOT NULL PRIMARY KEY,
    n      BIGINT NOT NULL
);

CREATE TABLE contacts.contact_tombstones (
    id         BLOB   NOT NULL PRIMARY KEY,
    owner_id   BLOB   NOT NULL,
    change_seq BIGINT NOT NULL,
    deleted_at TEXT   NOT NULL
);
CREATE INDEX contacts.idx_contacts_ct_tomb ON contact_tombstones(owner_id, change_seq);

CREATE TABLE contacts.label_tombstones (
    id         BLOB   NOT NULL PRIMARY KEY,
    owner_id   BLOB   NOT NULL,
    change_seq BIGINT NOT NULL,
    deleted_at TEXT   NOT NULL
);
CREATE INDEX contacts.idx_contacts_lbl_tomb ON label_tombstones(owner_id, change_seq);

CREATE TABLE contacts.group_tombstones (
    id         BLOB   NOT NULL PRIMARY KEY,
    owner_id   BLOB   NOT NULL,
    change_seq BIGINT NOT NULL,
    deleted_at TEXT   NOT NULL
);
CREATE INDEX contacts.idx_contacts_grp_tomb ON group_tombstones(owner_id, change_seq);

CREATE TABLE contacts.reminder_tombstones (
    id         BLOB   NOT NULL PRIMARY KEY,
    owner_id   BLOB   NOT NULL,
    change_seq BIGINT NOT NULL,
    deleted_at TEXT   NOT NULL
);
CREATE INDEX contacts.idx_contacts_rem_tomb ON reminder_tombstones(owner_id, change_seq);
