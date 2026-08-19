-- The staff directory is now read straight from the core's governed endpoints
-- (`/users/search` for browsing, `/internal/directory/users/:id` for resolving a
-- picked account). The local mirror below bypassed the instance sharing policy
-- (directory.enabled / share_email / audience), so it is dropped rather than left
-- as an ungoverned, now-unread copy of the account list that a future query could
-- silently reach for.
DROP TABLE IF EXISTS contacts.directory_profiles;
