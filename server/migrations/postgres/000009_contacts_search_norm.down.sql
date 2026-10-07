DROP INDEX IF EXISTS contacts.idx_contacts_org_norm;
DROP INDEX IF EXISTS contacts.idx_contacts_name_norm;
ALTER TABLE contacts.contacts DROP COLUMN IF EXISTS addr_norm;
ALTER TABLE contacts.contacts DROP COLUMN IF EXISTS notes_norm;
ALTER TABLE contacts.contacts DROP COLUMN IF EXISTS job_norm;
ALTER TABLE contacts.contacts DROP COLUMN IF EXISTS phone_norm;
ALTER TABLE contacts.contacts DROP COLUMN IF EXISTS email_norm;
ALTER TABLE contacts.contacts DROP COLUMN IF EXISTS org_norm;
ALTER TABLE contacts.contacts DROP COLUMN IF EXISTS name_norm;

-- The tsvector column and its trigger are not recreated: the trigger body
-- referenced `unaccent`, and this down migration is only a structural rollback.
ALTER TABLE contacts.contacts ADD COLUMN IF NOT EXISTS search_vector TSVECTOR;
CREATE INDEX IF NOT EXISTS idx_contacts_search ON contacts.contacts USING GIN(search_vector);
CREATE INDEX IF NOT EXISTS idx_contacts_emails ON contacts.contacts USING GIN(emails);
CREATE INDEX IF NOT EXISTS idx_contacts_phones ON contacts.contacts USING GIN(phones);
