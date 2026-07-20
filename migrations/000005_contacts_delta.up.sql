-- Delta primitives for the local-first pull (contacts, labels, groups, reminders):
-- monotonic change_seq + tombstones. contact_labels bump their contact (so its
-- inline label_ids re-sync); group_members bump their group (inline member_ids).

-- Generic bump helper set per table below (each needs its own sequence).

-- ===== contacts =====
CREATE SEQUENCE IF NOT EXISTS contacts.contact_change_seq;
ALTER TABLE contacts.contacts ADD COLUMN IF NOT EXISTS change_seq BIGINT NOT NULL DEFAULT nextval('contacts.contact_change_seq');
CREATE INDEX IF NOT EXISTS idx_contacts_change_seq ON contacts.contacts(owner_id, change_seq);
CREATE OR REPLACE FUNCTION contacts.bump_contact_change_seq() RETURNS trigger AS $$
BEGIN NEW.change_seq := nextval('contacts.contact_change_seq'); RETURN NEW; END;
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_contact_change_seq ON contacts.contacts;
CREATE TRIGGER trg_contact_change_seq BEFORE UPDATE ON contacts.contacts
    FOR EACH ROW EXECUTE FUNCTION contacts.bump_contact_change_seq();
CREATE TABLE IF NOT EXISTS contacts.contact_tombstones (
    id UUID PRIMARY KEY, owner_id UUID NOT NULL, change_seq BIGINT NOT NULL, deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW());
CREATE INDEX IF NOT EXISTS idx_contacts_ct_tomb ON contacts.contact_tombstones(owner_id, change_seq);
CREATE OR REPLACE FUNCTION contacts.contact_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO contacts.contact_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('contacts.contact_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END; $$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_contact_tombstone ON contacts.contacts;
CREATE TRIGGER trg_contact_tombstone AFTER DELETE ON contacts.contacts
    FOR EACH ROW EXECUTE FUNCTION contacts.contact_tombstone();

-- contact_labels → bump the contact (inline label_ids)
CREATE OR REPLACE FUNCTION contacts.cl_bump_contact() RETURNS trigger AS $$
BEGIN
    UPDATE contacts.contacts SET change_seq = change_seq
     WHERE id = COALESCE(NEW.contact_id, OLD.contact_id);
    RETURN COALESCE(NEW, OLD);
END; $$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_cl_bump_contact ON contacts.contact_labels;
CREATE TRIGGER trg_cl_bump_contact AFTER INSERT OR DELETE ON contacts.contact_labels
    FOR EACH ROW EXECUTE FUNCTION contacts.cl_bump_contact();

-- ===== labels =====
CREATE SEQUENCE IF NOT EXISTS contacts.label_change_seq;
ALTER TABLE contacts.labels ADD COLUMN IF NOT EXISTS change_seq BIGINT NOT NULL DEFAULT nextval('contacts.label_change_seq');
CREATE INDEX IF NOT EXISTS idx_contacts_label_change_seq ON contacts.labels(owner_id, change_seq);
CREATE OR REPLACE FUNCTION contacts.bump_label_change_seq() RETURNS trigger AS $$
BEGIN NEW.change_seq := nextval('contacts.label_change_seq'); RETURN NEW; END;
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_label_change_seq ON contacts.labels;
CREATE TRIGGER trg_label_change_seq BEFORE UPDATE ON contacts.labels
    FOR EACH ROW EXECUTE FUNCTION contacts.bump_label_change_seq();
CREATE TABLE IF NOT EXISTS contacts.label_tombstones (
    id UUID PRIMARY KEY, owner_id UUID NOT NULL, change_seq BIGINT NOT NULL, deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW());
CREATE INDEX IF NOT EXISTS idx_contacts_lbl_tomb ON contacts.label_tombstones(owner_id, change_seq);
CREATE OR REPLACE FUNCTION contacts.label_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO contacts.label_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('contacts.label_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END; $$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_label_tombstone ON contacts.labels;
CREATE TRIGGER trg_label_tombstone AFTER DELETE ON contacts.labels
    FOR EACH ROW EXECUTE FUNCTION contacts.label_tombstone();

-- ===== groups =====
CREATE SEQUENCE IF NOT EXISTS contacts.group_change_seq;
ALTER TABLE contacts.groups ADD COLUMN IF NOT EXISTS change_seq BIGINT NOT NULL DEFAULT nextval('contacts.group_change_seq');
CREATE INDEX IF NOT EXISTS idx_contacts_group_change_seq ON contacts.groups(owner_id, change_seq);
CREATE OR REPLACE FUNCTION contacts.bump_group_change_seq() RETURNS trigger AS $$
BEGIN NEW.change_seq := nextval('contacts.group_change_seq'); RETURN NEW; END;
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_group_change_seq ON contacts.groups;
CREATE TRIGGER trg_group_change_seq BEFORE UPDATE ON contacts.groups
    FOR EACH ROW EXECUTE FUNCTION contacts.bump_group_change_seq();
CREATE TABLE IF NOT EXISTS contacts.group_tombstones (
    id UUID PRIMARY KEY, owner_id UUID NOT NULL, change_seq BIGINT NOT NULL, deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW());
CREATE INDEX IF NOT EXISTS idx_contacts_grp_tomb ON contacts.group_tombstones(owner_id, change_seq);
CREATE OR REPLACE FUNCTION contacts.group_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO contacts.group_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('contacts.group_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END; $$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_group_tombstone ON contacts.groups;
CREATE TRIGGER trg_group_tombstone AFTER DELETE ON contacts.groups
    FOR EACH ROW EXECUTE FUNCTION contacts.group_tombstone();

-- group_members → bump the group (inline member_ids)
CREATE OR REPLACE FUNCTION contacts.gm_bump_group() RETURNS trigger AS $$
BEGIN
    UPDATE contacts.groups SET change_seq = change_seq
     WHERE id = COALESCE(NEW.group_id, OLD.group_id);
    RETURN COALESCE(NEW, OLD);
END; $$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_gm_bump_group ON contacts.group_members;
CREATE TRIGGER trg_gm_bump_group AFTER INSERT OR DELETE ON contacts.group_members
    FOR EACH ROW EXECUTE FUNCTION contacts.gm_bump_group();

-- ===== reminders =====
CREATE SEQUENCE IF NOT EXISTS contacts.reminder_change_seq;
ALTER TABLE contacts.reminders ADD COLUMN IF NOT EXISTS change_seq BIGINT NOT NULL DEFAULT nextval('contacts.reminder_change_seq');
CREATE INDEX IF NOT EXISTS idx_contacts_reminder_change_seq ON contacts.reminders(owner_id, change_seq);
CREATE OR REPLACE FUNCTION contacts.bump_reminder_change_seq() RETURNS trigger AS $$
BEGIN NEW.change_seq := nextval('contacts.reminder_change_seq'); RETURN NEW; END;
$$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_reminder_change_seq ON contacts.reminders;
CREATE TRIGGER trg_reminder_change_seq BEFORE UPDATE ON contacts.reminders
    FOR EACH ROW EXECUTE FUNCTION contacts.bump_reminder_change_seq();
CREATE TABLE IF NOT EXISTS contacts.reminder_tombstones (
    id UUID PRIMARY KEY, owner_id UUID NOT NULL, change_seq BIGINT NOT NULL, deleted_at TIMESTAMPTZ NOT NULL DEFAULT NOW());
CREATE INDEX IF NOT EXISTS idx_contacts_rem_tomb ON contacts.reminder_tombstones(owner_id, change_seq);
CREATE OR REPLACE FUNCTION contacts.reminder_tombstone() RETURNS trigger AS $$
BEGIN
    INSERT INTO contacts.reminder_tombstones (id, owner_id, change_seq)
    VALUES (OLD.id, OLD.owner_id, nextval('contacts.reminder_change_seq'))
    ON CONFLICT (id) DO UPDATE SET change_seq = EXCLUDED.change_seq, deleted_at = NOW();
    RETURN OLD;
END; $$ LANGUAGE plpgsql;
DROP TRIGGER IF EXISTS trg_reminder_tombstone ON contacts.reminders;
CREATE TRIGGER trg_reminder_tombstone AFTER DELETE ON contacts.reminders
    FOR EACH ROW EXECUTE FUNCTION contacts.reminder_tombstone();
