CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

CREATE COLLATION case_insensitive (provider = icu, locale = 'und-u-ks-level2', deterministic = false);

CREATE SCHEMA app;

CREATE OR REPLACE FUNCTION app.set_updated_at()
    returns trigger as
$$
BEGIN
    NEW.updated_at = now();
    return NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION app.trigger_updated_at(tablename regclass)
    returns void as
$$
BEGIN
    execute format('CREATE TRIGGER set_updated_at
        BEFORE UPDATE
        ON %s
        FOR EACH ROW
        WHEN (OLD is distinct from NEW)
    EXECUTE FUNCTION app.set_updated_at();', tablename);
END;
$$ LANGUAGE plpgsql;

CREATE TABLE app.user (
    id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    username text COLLATE "case_insensitive" UNIQUE NOT NULL,
    email text COLLATE "case_insensitive" UNIQUE NOT NULL,
    bio text NOT NULL DEFAULT '',
    image text NULL,
    password_hash text NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz
);

SELECT app.trigger_updated_at('app."user"');
