-- New feature surface for the contacts module: labels, bulk-friendly columns,
-- saved filters (smart views), reminders, public shares, change history,
-- ignored duplicate pairs, per-user settings and CardDAV access tokens.

-- ─── Extra columns on contacts ──────────────────────────────────────────────
ALTER TABLE contacts.contacts
    ADD COLUMN IF NOT EXISTS is_archived         BOOLEAN     NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS archived_at         TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS is_blocked          BOOLEAN     NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS last_interaction_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS interaction_count   INTEGER     NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS pronouns            VARCHAR(50);

CREATE INDEX IF NOT EXISTS idx_contacts_archived
    ON contacts.contacts(owner_id) WHERE is_archived = TRUE;
CREATE INDEX IF NOT EXISTS idx_contacts_last_interaction
    ON contacts.contacts(owner_id, last_interaction_at DESC NULLS LAST);

-- ─── Labels (colored tags, many-to-many) ────────────────────────────────────
CREATE TABLE IF NOT EXISTS contacts.labels (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id    UUID NOT NULL,
    name        VARCHAR(120) NOT NULL,
    color       VARCHAR(7)  NOT NULL DEFAULT '#5f6368',
    icon        VARCHAR(40),
    is_system   BOOLEAN     NOT NULL DEFAULT FALSE,
    position    INTEGER     NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, name)
);
CREATE INDEX IF NOT EXISTS idx_contacts_labels_owner ON contacts.labels(owner_id, position);

CREATE TABLE IF NOT EXISTS contacts.contact_labels (
    label_id    UUID NOT NULL REFERENCES contacts.labels(id)   ON DELETE CASCADE,
    contact_id  UUID NOT NULL REFERENCES contacts.contacts(id) ON DELETE CASCADE,
    added_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (label_id, contact_id)
);
CREATE INDEX IF NOT EXISTS idx_contacts_cl_contact ON contacts.contact_labels(contact_id);

CREATE TRIGGER labels_updated_at BEFORE UPDATE ON contacts.labels
    FOR EACH ROW EXECUTE FUNCTION contacts.set_updated_at();

-- ─── Saved filters (smart views) ────────────────────────────────────────────
CREATE TABLE IF NOT EXISTS contacts.saved_filters (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id    UUID NOT NULL,
    name        VARCHAR(120) NOT NULL,
    icon        VARCHAR(40)  NOT NULL DEFAULT 'Filter',
    color       VARCHAR(7)   NOT NULL DEFAULT '#1a73e8',
    criteria    JSONB        NOT NULL DEFAULT '{}',
    position    INTEGER      NOT NULL DEFAULT 0,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, name)
);
CREATE INDEX IF NOT EXISTS idx_contacts_filters_owner ON contacts.saved_filters(owner_id, position);

CREATE TRIGGER saved_filters_updated_at BEFORE UPDATE ON contacts.saved_filters
    FOR EACH ROW EXECUTE FUNCTION contacts.set_updated_at();

-- ─── Reminders (birthdays, follow-ups) ──────────────────────────────────────
CREATE TABLE IF NOT EXISTS contacts.reminders (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id    UUID NOT NULL,
    contact_id  UUID NOT NULL REFERENCES contacts.contacts(id) ON DELETE CASCADE,
    kind        VARCHAR(20)  NOT NULL DEFAULT 'follow_up', -- 'follow_up' | 'birthday' | 'custom'
    message     VARCHAR(500),
    remind_at   TIMESTAMPTZ  NOT NULL,
    recurrence  VARCHAR(20)  NOT NULL DEFAULT 'none',      -- 'none' | 'yearly'
    is_done     BOOLEAN      NOT NULL DEFAULT FALSE,
    notified_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_contacts_reminders_due
    ON contacts.reminders(remind_at) WHERE is_done = FALSE AND notified_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_contacts_reminders_owner
    ON contacts.reminders(owner_id, remind_at);

-- ─── Public shares (single contact or group) ────────────────────────────────
CREATE TABLE IF NOT EXISTS contacts.shares (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id      UUID NOT NULL,
    contact_id    UUID REFERENCES contacts.contacts(id) ON DELETE CASCADE,
    group_id      UUID REFERENCES contacts.groups(id)   ON DELETE CASCADE,
    token         VARCHAR(64) UNIQUE NOT NULL,
    permission    VARCHAR(10) NOT NULL DEFAULT 'view',
    expires_at    TIMESTAMPTZ,
    password_hash VARCHAR(255),
    max_accesses  INTEGER,
    access_count  INTEGER NOT NULL DEFAULT 0,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT share_target CHECK (contact_id IS NOT NULL OR group_id IS NOT NULL)
);
CREATE INDEX IF NOT EXISTS idx_contacts_shares_owner ON contacts.shares(owner_id);
CREATE INDEX IF NOT EXISTS idx_contacts_shares_token ON contacts.shares(token);

-- ─── Change history (field-level audit per contact) ─────────────────────────
CREATE TABLE IF NOT EXISTS contacts.change_log (
    id          BIGSERIAL PRIMARY KEY,
    contact_id  UUID NOT NULL REFERENCES contacts.contacts(id) ON DELETE CASCADE,
    owner_id    UUID NOT NULL,
    field       VARCHAR(60) NOT NULL,
    old_value   TEXT,
    new_value   TEXT,
    changed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_contacts_changelog ON contacts.change_log(contact_id, changed_at DESC);

-- ─── Ignored duplicate pairs (so the user can dismiss a suggestion) ──────────
CREATE TABLE IF NOT EXISTS contacts.dedup_ignored (
    owner_id    UUID NOT NULL,
    contact_a   UUID NOT NULL REFERENCES contacts.contacts(id) ON DELETE CASCADE,
    contact_b   UUID NOT NULL REFERENCES contacts.contacts(id) ON DELETE CASCADE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (owner_id, contact_a, contact_b)
);

-- ─── Per-user settings for the contacts module ──────────────────────────────
CREATE TABLE IF NOT EXISTS contacts.user_settings (
    owner_id    UUID PRIMARY KEY,
    prefs       JSONB NOT NULL DEFAULT '{}',
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── CardDAV access tokens (one row per user, hashed) ───────────────────────
CREATE TABLE IF NOT EXISTS contacts.carddav_tokens (
    owner_id    UUID PRIMARY KEY,
    token_hash  VARCHAR(64) NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_used_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_contacts_carddav_hash ON contacts.carddav_tokens(token_hash);
