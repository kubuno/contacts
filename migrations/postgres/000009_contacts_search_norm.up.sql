-- Move full-text search off PostgreSQL's `tsvector` / `to_tsvector` / `ts_rank`
-- / `unaccent` and onto `kubuno_db::search`: the searchable fields are reduced
-- to Snowball French stems and deaccented IN RUST at write time and stored in
-- plain `TEXT` columns, then a query is put through the same reduction and
-- matched with a portable `LIKE`. Because the stemming happens before any SQL,
-- the stored and searched tokens are byte-for-byte identical on PostgreSQL,
-- MySQL and SQLite. The MySQL and SQLite migrations declare the `*_norm`
-- columns from their CREATE TABLE; here the PostgreSQL table sheds its
-- `tsvector` machinery and gains the columns.
--
-- Two BEFORE INSERT/UPDATE triggers also go: `contacts_search_vector` (which
-- both filled `search_vector` AND derived `display_name` from the name parts)
-- and `contacts_update_etag`. Both behaviours move to Rust — the module now
-- computes `display_name`, the `*_norm` columns and the `etag` on every write —
-- because neither MySQL nor SQLite has the trigger form this used.
--
-- NOTE: the `unaccent` / `pg_trgm` extensions are NOT dropped — contacts never
-- created them (they are provided by the core's own migrations). What is lost:
-- `pg_trgm`-style typo tolerance (a `LIKE '%stem%'` needs the stem to appear as
-- a substring). Stemming still folds inflections and the normalizer folds
-- accents, so inflected and accented queries still match.

-- Retire the tsvector column, its GIN index and its trigger/function.
DROP TRIGGER IF EXISTS contacts_search_vector ON contacts.contacts;
DROP FUNCTION IF EXISTS contacts.update_search_vector();
DROP INDEX IF EXISTS contacts.idx_contacts_search;
ALTER TABLE contacts.contacts DROP COLUMN IF EXISTS search_vector;

-- Retire the etag trigger/function (etag is minted in Rust now).
DROP TRIGGER IF EXISTS contacts_update_etag ON contacts.contacts;
DROP FUNCTION IF EXISTS contacts.update_etag();

-- The jsonb GIN indexes on emails/phones were only there for the tsvector
-- trigger's array walks; nothing reads them now.
DROP INDEX IF EXISTS contacts.idx_contacts_emails;
DROP INDEX IF EXISTS contacts.idx_contacts_phones;

-- One normalized TEXT column per searchable field (the portable stand-in for
-- `setweight`/scoped operators): name (weight A), organization (B), e-mail (C),
-- phone (D), plus job title / notes / address for the scoped `job:`/`note:`/
-- `addr:` operators. Existing rows get an empty string; every save recomputes
-- them in Rust.
ALTER TABLE contacts.contacts ADD COLUMN IF NOT EXISTS name_norm  TEXT NOT NULL DEFAULT '';
ALTER TABLE contacts.contacts ADD COLUMN IF NOT EXISTS org_norm   TEXT NOT NULL DEFAULT '';
ALTER TABLE contacts.contacts ADD COLUMN IF NOT EXISTS email_norm TEXT NOT NULL DEFAULT '';
ALTER TABLE contacts.contacts ADD COLUMN IF NOT EXISTS phone_norm TEXT NOT NULL DEFAULT '';
ALTER TABLE contacts.contacts ADD COLUMN IF NOT EXISTS job_norm   TEXT NOT NULL DEFAULT '';
ALTER TABLE contacts.contacts ADD COLUMN IF NOT EXISTS notes_norm TEXT NOT NULL DEFAULT '';
ALTER TABLE contacts.contacts ADD COLUMN IF NOT EXISTS addr_norm  TEXT NOT NULL DEFAULT '';

CREATE INDEX IF NOT EXISTS idx_contacts_name_norm  ON contacts.contacts(name_norm);
CREATE INDEX IF NOT EXISTS idx_contacts_org_norm   ON contacts.contacts(org_norm);
