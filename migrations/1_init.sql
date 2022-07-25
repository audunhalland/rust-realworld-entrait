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
    user_id uuid PRIMARY KEY DEFAULT uuid_generate_v4(),
    username text COLLATE "case_insensitive" UNIQUE NOT NULL,
    email text COLLATE "case_insensitive" UNIQUE NOT NULL,
    bio text NOT NULL DEFAULT '',
    image text NULL,
    password_hash text NOT NULL,

    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz
);

SELECT app.trigger_updated_at('app."user"');

create table app.follow
(
    followed_user_id uuid NOT NULL references app.user (user_id) on delete cascade,
    following_user_id uuid NOT NULL references app.user (user_id) on delete cascade,
    created_at timestamptz NOT NULL default now(),
    updated_at timestamptz,

    constraint user_cannot_follow_self check (followed_user_id != following_user_id),
    primary key (following_user_id, followed_user_id)
);

SELECT app.trigger_updated_at('app."follow"');

create table app.article
(
    article_id uuid PRIMARY KEY DEFAULT uuid_generate_v1mc(),
    user_id uuid NOT NULL REFERENCES app.user (user_id) ON DELETE CASCADE,
    slug text UNIQUE NOT NULL,
    title text NOT NULL,
    description text NOT NULL,
    body text NOT NULL,
    tag_list text[] NOT NULL,

    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz NOT NULL DEFAULT now()
);

SELECT app.trigger_updated_at('app."article"');

-- This should speed up searching with tags.
CREATE INDEX article_tags_gin ON app.article USING gin (tag_list);

-- This table is much more clearly a cousin table of `article` so it's named as such.
CREATE TABLE app.article_favorite
(
    article_id uuid NOT NULL REFERENCES app.article (article_id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES app.user (user_id) ON DELETE CASCADE,

    created_at timestamptz NOT NULL DEFAULT now(),
    updated_at timestamptz,
    primary key (article_id, user_id)
);

SELECT app.trigger_updated_at('app."article_favorite"');

CREATE TABLE app.article_comment
(
    comment_id bigserial PRIMARY KEY,
    article_id uuid NOT NULL REFERENCES app.article (article_id) ON DELETE CASCADE,
    user_id uuid NOT NULL REFERENCES app.user (user_id) ON DELETE CASCADE,
    body text NOT NULL,

    created_at timestamptz NOT NULL default now(),
    updated_at timestamptz NOT NULL default now()
);

SELECT app.trigger_updated_at('app."article_comment"');

-- This is going to be the primary lookup method so it's not a bad idea to pre-emptively create an index for it,
-- as Postgres wouldn't otherwise do it by default.
CREATE INDEX ON app.article_comment (article_id, created_at);
