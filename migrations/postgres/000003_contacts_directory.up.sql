-- Annuaire d'instance : profils des utilisateurs Kubuno partagés
CREATE TABLE IF NOT EXISTS contacts.directory_profiles (
    kubuno_user_id  UUID PRIMARY KEY,
    display_name    VARCHAR(500) NOT NULL DEFAULT '',
    email           VARCHAR(500) NOT NULL DEFAULT '',
    avatar_url      TEXT,
    department      VARCHAR(255),
    job_title       VARCHAR(255),
    phone           VARCHAR(100),
    is_visible      BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dir_profiles_visible ON contacts.directory_profiles(is_visible)
    WHERE is_visible = TRUE;

-- GIN index sur tsvector pour la recherche plein-texte.
-- unaccent() n'est pas IMMUTABLE donc on ne peut pas l'utiliser dans l'expression d'index ;
-- on utilise 'simple' (tokenisation sans stemming) qui est entièrement déterministe.
CREATE INDEX IF NOT EXISTS idx_dir_profiles_search ON contacts.directory_profiles
    USING GIN(to_tsvector('simple', COALESCE(display_name, '') || ' ' || COALESCE(email, '')));
