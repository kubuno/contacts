DROP TABLE IF EXISTS contacts.carddav_tokens;
DROP TABLE IF EXISTS contacts.user_settings;
DROP TABLE IF EXISTS contacts.dedup_ignored;
DROP TABLE IF EXISTS contacts.change_log;
DROP TABLE IF EXISTS contacts.shares;
DROP TABLE IF EXISTS contacts.reminders;
DROP TABLE IF EXISTS contacts.saved_filters;
DROP TABLE IF EXISTS contacts.contact_labels;
DROP TABLE IF EXISTS contacts.labels;

ALTER TABLE contacts.contacts
    DROP COLUMN IF EXISTS is_archived,
    DROP COLUMN IF EXISTS archived_at,
    DROP COLUMN IF EXISTS is_blocked,
    DROP COLUMN IF EXISTS last_interaction_at,
    DROP COLUMN IF EXISTS interaction_count,
    DROP COLUMN IF EXISTS pronouns;
