-- The shared address book of the instance.
--
-- Personal contacts belong to an account (`owner_id`) and nobody else sees them.
-- This table is the other half an organisation needs: outside contacts —
-- suppliers, partners, emergency numbers — entered once by the administration
-- and readable by every user. Nothing here is owned by a user, and no user
-- writes to it; the module refuses any write from a non-admin.
--
-- It is deliberately NOT a copy of the account directory: accounts stay in
-- `core.users` under the core's `directory.*` policy (migration 000006 dropped
-- this module's mirror of them for that very reason). This table only ever
-- holds people who have no account.

CREATE TABLE IF NOT EXISTS contacts.shared_contacts (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    display_name  VARCHAR(500) NOT NULL,
    organization  VARCHAR(255),
    job_title     VARCHAR(255),
    email         VARCHAR(320),
    phone         VARCHAR(64),
    notes         TEXT,
    -- The administrator who last wrote the entry, for accountability. Kept even
    -- if that account disappears (no FK): the entry belongs to the instance.
    updated_by    UUID,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_shared_contacts_name
    ON contacts.shared_contacts (LOWER(display_name));

CREATE INDEX IF NOT EXISTS idx_shared_contacts_org
    ON contacts.shared_contacts (LOWER(COALESCE(organization, '')));

DROP TRIGGER IF EXISTS trg_shared_contacts_updated_at ON contacts.shared_contacts;
CREATE TRIGGER trg_shared_contacts_updated_at
    BEFORE UPDATE ON contacts.shared_contacts
    FOR EACH ROW EXECUTE FUNCTION contacts.set_updated_at();
