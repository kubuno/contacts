CREATE TABLE contacts.groups (
    id          UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id    UUID NOT NULL,
    name        VARCHAR(255) NOT NULL,
    color       VARCHAR(7) NOT NULL DEFAULT '#1a73e8',
    is_system   BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, name)
);

CREATE TABLE contacts.group_members (
    group_id    UUID NOT NULL REFERENCES contacts.groups(id) ON DELETE CASCADE,
    contact_id  UUID NOT NULL REFERENCES contacts.contacts(id) ON DELETE CASCADE,
    added_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (group_id, contact_id)
);

CREATE INDEX idx_contacts_groups_owner   ON contacts.groups(owner_id);
CREATE INDEX idx_contacts_gm_contact     ON contacts.group_members(contact_id);

CREATE TRIGGER groups_updated_at
    BEFORE UPDATE ON contacts.groups
    FOR EACH ROW EXECUTE FUNCTION contacts.set_updated_at();
