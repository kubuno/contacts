DROP TABLE IF EXISTS contacts.interaction_log;
DROP TRIGGER IF EXISTS contacts_update_etag ON contacts.contacts;
DROP TRIGGER IF EXISTS contacts_updated_at ON contacts.contacts;
DROP TRIGGER IF EXISTS contacts_search_vector ON contacts.contacts;
DROP TABLE IF EXISTS contacts.contacts;
DROP FUNCTION IF EXISTS contacts.update_etag();
DROP FUNCTION IF EXISTS contacts.update_search_vector();
DROP FUNCTION IF EXISTS contacts.set_updated_at();
DROP SCHEMA IF EXISTS contacts;
