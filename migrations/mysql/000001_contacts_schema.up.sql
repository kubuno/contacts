-- MySQL / MariaDB — the `contacts` database is created by kubuno-db's schema
-- setup before the migrator runs, so there is no CREATE DATABASE here. This
-- single file declares the FINAL shape the PostgreSQL side reached across its
-- 000001..000010 migrations (directory mirror dropped, delta journal,
-- normalized search columns).
--
-- Differences from PostgreSQL, and why:
--   * UUID -> BINARY(16): what sqlx encodes a `uuid::Uuid` as on MySQL.
--   * No DEFAULT on `id`: MySQL has no gen_random_uuid() and no RETURNING, so
--     the process supplies every primary key.
--   * TIMESTAMPTZ -> DATETIME(6); every value written is UTC (the pool pins
--     `time_zone = '+00:00'`). updated_at uses ON UPDATE CURRENT_TIMESTAMP(6).
--   * JSONB -> JSON (the contact field arrays and the settings/criteria blobs).
--   * Full-text search is the normalized-column form (name_norm / org_norm /
--     email_norm / phone_norm / job_norm / notes_norm / addr_norm, filled in
--     Rust): no tsvector, no GIN, no unaccent. display_name and etag are also
--     computed in Rust (their PostgreSQL triggers are gone).
--   * The delta layer is the journal (change_counter + per-row change_seq) plus
--     the four tombstone tables; no sequences, no triggers.
--   * Partial indexes (WHERE ...) become plain indexes (MySQL has none).
--   * utf8mb4_bin so a UNIQUE key stays case- and accent-sensitive.

CREATE TABLE contacts (
    id               BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id         BINARY(16)   NOT NULL,
    given_name       VARCHAR(255),
    middle_name      VARCHAR(255),
    family_name      VARCHAR(255),
    name_prefix      VARCHAR(50),
    name_suffix      VARCHAR(50),
    nickname         VARCHAR(255),
    display_name     VARCHAR(500) NOT NULL DEFAULT '',
    organization     VARCHAR(255),
    department       VARCHAR(255),
    job_title        VARCHAR(255),
    avatar_path      TEXT,
    avatar_color     VARCHAR(7)   NOT NULL DEFAULT '#1a73e8',
    emails           JSON         NOT NULL,
    phones           JSON         NOT NULL,
    addresses        JSON         NOT NULL,
    urls             JSON         NOT NULL,
    dates            JSON         NOT NULL,
    relations        JSON         NOT NULL,
    instant_messages JSON         NOT NULL,
    custom_fields    JSON         NOT NULL,
    notes            TEXT,
    is_starred       BOOLEAN      NOT NULL DEFAULT FALSE,
    is_trashed       BOOLEAN      NOT NULL DEFAULT FALSE,
    trashed_at       DATETIME(6),
    kubuno_user_id   BINARY(16),
    is_archived      BOOLEAN      NOT NULL DEFAULT FALSE,
    archived_at      DATETIME(6),
    is_blocked       BOOLEAN      NOT NULL DEFAULT FALSE,
    last_interaction_at DATETIME(6),
    interaction_count   INT       NOT NULL DEFAULT 0,
    pronouns         VARCHAR(50),
    vcard_uid        VARCHAR(500) NOT NULL,
    etag             VARCHAR(64)  NOT NULL DEFAULT '',
    import_source    VARCHAR(20)  NOT NULL DEFAULT 'manual',
    name_norm        TEXT         NOT NULL,
    org_norm         TEXT         NOT NULL,
    email_norm       TEXT         NOT NULL,
    phone_norm       TEXT         NOT NULL,
    job_norm         TEXT         NOT NULL,
    notes_norm       TEXT         NOT NULL,
    addr_norm        TEXT         NOT NULL,
    change_seq       BIGINT       NOT NULL DEFAULT 0,
    created_at       DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at       DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                                  ON UPDATE CURRENT_TIMESTAMP(6),
    UNIQUE (vcard_uid)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_owner       ON contacts(owner_id);
CREATE INDEX idx_contacts_trashed     ON contacts(owner_id, is_trashed);
CREATE INDEX idx_contacts_display     ON contacts(owner_id, display_name);
CREATE INDEX idx_contacts_vcard_uid   ON contacts(vcard_uid);
CREATE INDEX idx_contacts_kubuno_user ON contacts(kubuno_user_id);
CREATE INDEX idx_contacts_change_seq  ON contacts(owner_id, change_seq);
CREATE INDEX idx_contacts_last_interaction ON contacts(owner_id, last_interaction_at);
CREATE INDEX idx_contacts_name_norm   ON contacts(name_norm(191));
CREATE INDEX idx_contacts_org_norm    ON contacts(org_norm(191));

CREATE TABLE interaction_log (
    id               BINARY(16)  NOT NULL PRIMARY KEY,
    contact_id       BINARY(16)  NOT NULL,
    owner_id         BINARY(16)  NOT NULL,
    interaction_type VARCHAR(20) NOT NULL,
    summary          VARCHAR(500),
    source_module    VARCHAR(20),
    source_id        BINARY(16),
    occurred_at      DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_interactions ON interaction_log(contact_id, occurred_at);

CREATE TABLE `groups` (
    id          BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id    BINARY(16)   NOT NULL,
    name        VARCHAR(255) NOT NULL,
    color       VARCHAR(7)   NOT NULL DEFAULT '#1a73e8',
    is_system   BOOLEAN      NOT NULL DEFAULT FALSE,
    change_seq  BIGINT       NOT NULL DEFAULT 0,
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                             ON UPDATE CURRENT_TIMESTAMP(6),
    UNIQUE (owner_id, name)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_groups_owner      ON `groups`(owner_id);
CREATE INDEX idx_contacts_group_change_seq  ON `groups`(owner_id, change_seq);

CREATE TABLE group_members (
    group_id    BINARY(16)  NOT NULL,
    contact_id  BINARY(16)  NOT NULL,
    added_at    DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (group_id, contact_id),
    FOREIGN KEY (group_id)   REFERENCES `groups`(id)  ON DELETE CASCADE,
    FOREIGN KEY (contact_id) REFERENCES contacts(id)  ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_gm_contact ON group_members(contact_id);

CREATE TABLE labels (
    id          BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id    BINARY(16)   NOT NULL,
    name        VARCHAR(120) NOT NULL,
    color       VARCHAR(7)   NOT NULL DEFAULT '#5f6368',
    icon        VARCHAR(40),
    is_system   BOOLEAN      NOT NULL DEFAULT FALSE,
    position    INT          NOT NULL DEFAULT 0,
    change_seq  BIGINT       NOT NULL DEFAULT 0,
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                             ON UPDATE CURRENT_TIMESTAMP(6),
    UNIQUE (owner_id, name)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_labels_owner     ON labels(owner_id, position);
CREATE INDEX idx_contacts_label_change_seq ON labels(owner_id, change_seq);

CREATE TABLE contact_labels (
    label_id    BINARY(16)  NOT NULL,
    contact_id  BINARY(16)  NOT NULL,
    added_at    DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (label_id, contact_id),
    FOREIGN KEY (label_id)   REFERENCES labels(id)    ON DELETE CASCADE,
    FOREIGN KEY (contact_id) REFERENCES contacts(id)  ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_cl_contact ON contact_labels(contact_id);

CREATE TABLE saved_filters (
    id          BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id    BINARY(16)   NOT NULL,
    name        VARCHAR(120) NOT NULL,
    icon        VARCHAR(40)  NOT NULL DEFAULT 'Filter',
    color       VARCHAR(7)   NOT NULL DEFAULT '#1a73e8',
    criteria    JSON         NOT NULL,
    position    INT          NOT NULL DEFAULT 0,
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                             ON UPDATE CURRENT_TIMESTAMP(6),
    UNIQUE (owner_id, name)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_filters_owner ON saved_filters(owner_id, position);

CREATE TABLE reminders (
    id          BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id    BINARY(16)   NOT NULL,
    contact_id  BINARY(16)   NOT NULL,
    kind        VARCHAR(20)  NOT NULL DEFAULT 'follow_up',
    message     VARCHAR(500),
    remind_at   DATETIME(6)  NOT NULL,
    recurrence  VARCHAR(20)  NOT NULL DEFAULT 'none',
    is_done     BOOLEAN      NOT NULL DEFAULT FALSE,
    notified_at DATETIME(6),
    change_seq  BIGINT       NOT NULL DEFAULT 0,
    created_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_reminders_due       ON reminders(remind_at);
CREATE INDEX idx_contacts_reminders_owner     ON reminders(owner_id, remind_at);
CREATE INDEX idx_contacts_reminder_change_seq ON reminders(owner_id, change_seq);

CREATE TABLE shares (
    id            BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id      BINARY(16)  NOT NULL,
    contact_id    BINARY(16),
    group_id      BINARY(16),
    token         VARCHAR(64) NOT NULL,
    permission    VARCHAR(10) NOT NULL DEFAULT 'view',
    expires_at    DATETIME(6),
    password_hash VARCHAR(255),
    max_accesses  INT,
    access_count  INT         NOT NULL DEFAULT 0,
    created_at    DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    UNIQUE (token),
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE,
    FOREIGN KEY (group_id)   REFERENCES `groups`(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_shares_owner ON shares(owner_id);
CREATE INDEX idx_contacts_shares_token ON shares(token);

CREATE TABLE change_log (
    id          BIGINT      NOT NULL AUTO_INCREMENT PRIMARY KEY,
    contact_id  BINARY(16)  NOT NULL,
    owner_id    BINARY(16)  NOT NULL,
    field       VARCHAR(60) NOT NULL,
    old_value   TEXT,
    new_value   TEXT,
    changed_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_changelog ON change_log(contact_id, changed_at);

CREATE TABLE dedup_ignored (
    owner_id    BINARY(16)  NOT NULL,
    contact_a   BINARY(16)  NOT NULL,
    contact_b   BINARY(16)  NOT NULL,
    created_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    PRIMARY KEY (owner_id, contact_a, contact_b),
    FOREIGN KEY (contact_a) REFERENCES contacts(id) ON DELETE CASCADE,
    FOREIGN KEY (contact_b) REFERENCES contacts(id) ON DELETE CASCADE
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE user_settings (
    owner_id    BINARY(16)  NOT NULL PRIMARY KEY,
    prefs       JSON        NOT NULL,
    updated_at  DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                            ON UPDATE CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE carddav_tokens (
    owner_id     BINARY(16)  NOT NULL PRIMARY KEY,
    token_hash   VARCHAR(64) NOT NULL,
    created_at   DATETIME(6) NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    last_used_at DATETIME(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_carddav_hash ON carddav_tokens(token_hash);

CREATE TABLE shared_contacts (
    id            BINARY(16)   NOT NULL PRIMARY KEY,
    display_name  VARCHAR(500) NOT NULL,
    organization  VARCHAR(255),
    job_title     VARCHAR(255),
    email         VARCHAR(320),
    phone         VARCHAR(64),
    notes         TEXT,
    updated_by    BINARY(16),
    created_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    updated_at    DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6)
                               ON UPDATE CURRENT_TIMESTAMP(6)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_shared_contacts_name ON shared_contacts(display_name);

CREATE TABLE other_contacts (
    id            BINARY(16)   NOT NULL PRIMARY KEY,
    owner_id      BINARY(16)   NOT NULL,
    kind          VARCHAR(20)  NOT NULL,
    value         VARCHAR(320) NOT NULL,
    display_name  VARCHAR(255),
    source_module VARCHAR(32)  NOT NULL,
    first_seen_at DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    last_seen_at  DATETIME(6)  NOT NULL DEFAULT CURRENT_TIMESTAMP(6),
    seen_count    INT          NOT NULL DEFAULT 1 CHECK (seen_count > 0),
    dismissed_at  DATETIME(6),
    CONSTRAINT contacts_other_unique UNIQUE (owner_id, kind, value)
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_other_owner ON other_contacts(owner_id, last_seen_at);

-- ── Delta journal (portable change layer) ────────────────────────────────────
CREATE TABLE change_counter (
    domain VARCHAR(190) NOT NULL PRIMARY KEY,
    n      BIGINT       NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;

CREATE TABLE contact_tombstones (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id   BINARY(16)  NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at DATETIME(6) NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_ct_tomb ON contact_tombstones(owner_id, change_seq);

CREATE TABLE label_tombstones (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id   BINARY(16)  NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at DATETIME(6) NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_lbl_tomb ON label_tombstones(owner_id, change_seq);

CREATE TABLE group_tombstones (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id   BINARY(16)  NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at DATETIME(6) NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_grp_tomb ON group_tombstones(owner_id, change_seq);

CREATE TABLE reminder_tombstones (
    id         BINARY(16)  NOT NULL PRIMARY KEY,
    owner_id   BINARY(16)  NOT NULL,
    change_seq BIGINT      NOT NULL,
    deleted_at DATETIME(6) NOT NULL
) DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_bin;
CREATE INDEX idx_contacts_rem_tomb ON reminder_tombstones(owner_id, change_seq);
