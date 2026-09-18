-- "Other contacts": people the user has actually dealt with — a mail
-- correspondent, someone met in a chat — who were never saved into the address
-- book. They are NOT contacts: no photo, no groups, no vCard, no CardDAV. They
-- are a memory of interlocutors, kept so the user can promote the ones worth
-- keeping and ignore the rest.
--
-- Rows are pushed by the modules that handle interlocutors (mail, chat…) and
-- upserted on (owner, kind, value), so seeing the same person again only bumps
-- the counters rather than piling duplicates.
CREATE TABLE contacts.other_contacts (
    id            UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id      UUID NOT NULL,
    -- What `value` holds: 'email' | 'phone' | 'handle' (a module-specific
    -- identifier, e.g. a chat account). Kept open on purpose — a new module
    -- brings its own kind without a migration.
    kind          VARCHAR(20)  NOT NULL,
    -- Normalised by the writer: an address is lower-cased and trimmed, so the
    -- uniqueness below actually catches the same person twice.
    value         VARCHAR(320) NOT NULL,
    display_name  VARCHAR(255),
    -- The module that saw this person last; shown so the user knows where the
    -- name comes from.
    source_module VARCHAR(32)  NOT NULL,
    first_seen_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    last_seen_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    seen_count    INTEGER      NOT NULL DEFAULT 1 CHECK (seen_count > 0),
    -- Set when the user says "not interesting": the row stays so the next
    -- message does not resurrect the suggestion, but it is never listed again.
    dismissed_at  TIMESTAMPTZ,
    CONSTRAINT contacts_other_unique UNIQUE (owner_id, kind, value)
);

-- The list view: one owner's suggestions, most recently seen first.
CREATE INDEX idx_contacts_other_owner
    ON contacts.other_contacts (owner_id, last_seen_at DESC)
    WHERE dismissed_at IS NULL;
