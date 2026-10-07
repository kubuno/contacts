-- Move the delta layer off PostgreSQL sequences + triggers and onto the
-- application-driven `kubuno_db::journal` primitive (one shared counter row per
-- domain, seqs taken in Rust at write time, tombstones written in the same
-- transaction). Neither the sequence nor the trigger mechanism has a portable
-- form on MySQL/SQLite, so it is retired here on PostgreSQL too; the four
-- tombstone TABLES keep their exact 000005 shape (no data migration), only
-- their triggers go.
--
-- The `contacts_updated_at` / `groups_updated_at` / `labels_updated_at` /
-- `saved_filters_updated_at` / `trg_shared_contacts_updated_at` triggers from
-- 000001/000002/000004/000007 are deliberately LEFT in place (MySQL uses ON
-- UPDATE, SQLite sets it in Rust); only the change-seq, tombstone and
-- child-bump machinery is removed.

-- ── contacts: BEFORE UPDATE seq, AFTER DELETE tombstone, contact_labels bump ──
DROP TRIGGER IF EXISTS trg_contact_change_seq ON contacts.contacts;
DROP TRIGGER IF EXISTS trg_contact_tombstone  ON contacts.contacts;
DROP TRIGGER IF EXISTS trg_cl_bump_contact    ON contacts.contact_labels;
DROP FUNCTION IF EXISTS contacts.bump_contact_change_seq();
DROP FUNCTION IF EXISTS contacts.contact_tombstone();
DROP FUNCTION IF EXISTS contacts.cl_bump_contact();

-- ── labels ────────────────────────────────────────────────────────────────
DROP TRIGGER IF EXISTS trg_label_change_seq ON contacts.labels;
DROP TRIGGER IF EXISTS trg_label_tombstone  ON contacts.labels;
DROP FUNCTION IF EXISTS contacts.bump_label_change_seq();
DROP FUNCTION IF EXISTS contacts.label_tombstone();

-- ── groups: BEFORE UPDATE seq, AFTER DELETE tombstone, group_members bump ────
DROP TRIGGER IF EXISTS trg_group_change_seq ON contacts.groups;
DROP TRIGGER IF EXISTS trg_group_tombstone  ON contacts.groups;
DROP TRIGGER IF EXISTS trg_gm_bump_group    ON contacts.group_members;
DROP FUNCTION IF EXISTS contacts.bump_group_change_seq();
DROP FUNCTION IF EXISTS contacts.group_tombstone();
DROP FUNCTION IF EXISTS contacts.gm_bump_group();

-- ── reminders ───────────────────────────────────────────────────────────────
DROP TRIGGER IF EXISTS trg_reminder_change_seq ON contacts.reminders;
DROP TRIGGER IF EXISTS trg_reminder_tombstone  ON contacts.reminders;
DROP FUNCTION IF EXISTS contacts.bump_reminder_change_seq();
DROP FUNCTION IF EXISTS contacts.reminder_tombstone();

-- The DEFAULTs reference the sequences, so they must go before the sequences do.
ALTER TABLE contacts.contacts  ALTER COLUMN change_seq SET DEFAULT 0;
ALTER TABLE contacts.labels    ALTER COLUMN change_seq SET DEFAULT 0;
ALTER TABLE contacts.groups    ALTER COLUMN change_seq SET DEFAULT 0;
ALTER TABLE contacts.reminders ALTER COLUMN change_seq SET DEFAULT 0;
DROP SEQUENCE IF EXISTS contacts.contact_change_seq;
DROP SEQUENCE IF EXISTS contacts.label_change_seq;
DROP SEQUENCE IF EXISTS contacts.group_change_seq;
DROP SEQUENCE IF EXISTS contacts.reminder_change_seq;

-- ── The journal's shared counter, seeded to continue the existing sequences ───
CREATE TABLE IF NOT EXISTS contacts.change_counter (
    domain VARCHAR(190) NOT NULL PRIMARY KEY,
    n      BIGINT       NOT NULL
);

-- Seed each domain to the current max (live rows and tombstones both) so
-- `next_seq` (n := n + 1) never hands out a value an existing row already holds.
INSERT INTO contacts.change_counter (domain, n)
    SELECT 'contacts', GREATEST(
        COALESCE((SELECT MAX(change_seq) FROM contacts.contacts), 0),
        COALESCE((SELECT MAX(change_seq) FROM contacts.contact_tombstones), 0))
    ON CONFLICT (domain) DO NOTHING;
INSERT INTO contacts.change_counter (domain, n)
    SELECT 'labels', GREATEST(
        COALESCE((SELECT MAX(change_seq) FROM contacts.labels), 0),
        COALESCE((SELECT MAX(change_seq) FROM contacts.label_tombstones), 0))
    ON CONFLICT (domain) DO NOTHING;
INSERT INTO contacts.change_counter (domain, n)
    SELECT 'groups', GREATEST(
        COALESCE((SELECT MAX(change_seq) FROM contacts.groups), 0),
        COALESCE((SELECT MAX(change_seq) FROM contacts.group_tombstones), 0))
    ON CONFLICT (domain) DO NOTHING;
INSERT INTO contacts.change_counter (domain, n)
    SELECT 'reminders', GREATEST(
        COALESCE((SELECT MAX(change_seq) FROM contacts.reminders), 0),
        COALESCE((SELECT MAX(change_seq) FROM contacts.reminder_tombstones), 0))
    ON CONFLICT (domain) DO NOTHING;
