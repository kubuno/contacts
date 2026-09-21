CREATE SCHEMA IF NOT EXISTS contacts;

CREATE OR REPLACE FUNCTION contacts.set_updated_at()
RETURNS TRIGGER AS $$
BEGIN NEW.updated_at = NOW(); RETURN NEW; END;
$$ LANGUAGE plpgsql;

CREATE TABLE contacts.contacts (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    owner_id        UUID NOT NULL,

    given_name      VARCHAR(255),
    middle_name     VARCHAR(255),
    family_name     VARCHAR(255),
    name_prefix     VARCHAR(50),
    name_suffix     VARCHAR(50),
    nickname        VARCHAR(255),
    display_name    VARCHAR(500) NOT NULL DEFAULT '',

    organization    VARCHAR(255),
    department      VARCHAR(255),
    job_title       VARCHAR(255),

    avatar_path     TEXT,
    avatar_color    VARCHAR(7) NOT NULL DEFAULT '#1a73e8',

    emails          JSONB NOT NULL DEFAULT '[]',
    phones          JSONB NOT NULL DEFAULT '[]',
    addresses       JSONB NOT NULL DEFAULT '[]',
    urls            JSONB NOT NULL DEFAULT '[]',
    dates           JSONB NOT NULL DEFAULT '[]',
    relations       JSONB NOT NULL DEFAULT '[]',
    instant_messages JSONB NOT NULL DEFAULT '[]',
    custom_fields   JSONB NOT NULL DEFAULT '[]',

    notes           TEXT,

    is_starred      BOOLEAN NOT NULL DEFAULT FALSE,
    is_trashed      BOOLEAN NOT NULL DEFAULT FALSE,
    trashed_at      TIMESTAMPTZ,
    kubuno_user_id  UUID,

    vcard_uid       VARCHAR(500) UNIQUE NOT NULL DEFAULT uuid_generate_v4()::text,
    etag            VARCHAR(64)  NOT NULL DEFAULT md5(random()::text),

    search_vector   TSVECTOR,

    import_source   VARCHAR(20) NOT NULL DEFAULT 'manual',

    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_contacts_owner       ON contacts.contacts(owner_id);
CREATE INDEX idx_contacts_starred     ON contacts.contacts(owner_id) WHERE is_starred = TRUE;
CREATE INDEX idx_contacts_trashed     ON contacts.contacts(owner_id, is_trashed);
CREATE INDEX idx_contacts_display     ON contacts.contacts(owner_id, display_name ASC);
CREATE INDEX idx_contacts_search      ON contacts.contacts USING GIN(search_vector);
CREATE INDEX idx_contacts_vcard_uid   ON contacts.contacts(vcard_uid);
CREATE INDEX idx_contacts_kubuno_user ON contacts.contacts(kubuno_user_id) WHERE kubuno_user_id IS NOT NULL;
CREATE INDEX idx_contacts_emails      ON contacts.contacts USING GIN(emails);
CREATE INDEX idx_contacts_phones      ON contacts.contacts USING GIN(phones);

CREATE OR REPLACE FUNCTION contacts.update_search_vector()
RETURNS TRIGGER AS $$
DECLARE
    emails_text TEXT;
    phones_text TEXT;
BEGIN
    SELECT string_agg(email->>'value', ' ')
    INTO emails_text
    FROM jsonb_array_elements(NEW.emails) AS email;

    SELECT string_agg(phone->>'value', ' ')
    INTO phones_text
    FROM jsonb_array_elements(NEW.phones) AS phone;

    NEW.search_vector :=
        setweight(to_tsvector('simple', unaccent(COALESCE(NEW.display_name, ''))), 'A') ||
        setweight(to_tsvector('simple', unaccent(COALESCE(NEW.organization, ''))), 'B') ||
        setweight(to_tsvector('simple', COALESCE(emails_text, '')), 'C') ||
        setweight(to_tsvector('simple', COALESCE(phones_text, '')), 'D');

    IF NEW.display_name = '' OR NEW.display_name IS NULL THEN
        NEW.display_name := TRIM(
            COALESCE(NEW.name_prefix || ' ', '') ||
            COALESCE(NEW.given_name  || ' ', '') ||
            COALESCE(NEW.middle_name || ' ', '') ||
            COALESCE(NEW.family_name, '')
        );
        IF NEW.display_name = '' OR NEW.display_name IS NULL THEN
            NEW.display_name := COALESCE(NEW.nickname, NEW.organization, 'Contact sans nom');
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER contacts_search_vector
    BEFORE INSERT OR UPDATE OF given_name, family_name, organization,
                               display_name, emails, phones ON contacts.contacts
    FOR EACH ROW EXECUTE FUNCTION contacts.update_search_vector();

CREATE TRIGGER contacts_updated_at
    BEFORE UPDATE ON contacts.contacts
    FOR EACH ROW EXECUTE FUNCTION contacts.set_updated_at();

CREATE OR REPLACE FUNCTION contacts.update_etag()
RETURNS TRIGGER AS $$
BEGIN
    NEW.etag := md5(random()::text || clock_timestamp()::text);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER contacts_update_etag
    BEFORE UPDATE ON contacts.contacts
    FOR EACH ROW EXECUTE FUNCTION contacts.update_etag();

CREATE TABLE contacts.interaction_log (
    id               UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    contact_id       UUID NOT NULL REFERENCES contacts.contacts(id) ON DELETE CASCADE,
    owner_id         UUID NOT NULL,
    interaction_type VARCHAR(20) NOT NULL,
    summary          VARCHAR(500),
    source_module    VARCHAR(20),
    source_id        UUID,
    occurred_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_contacts_interactions ON contacts.interaction_log(contact_id, occurred_at DESC);
