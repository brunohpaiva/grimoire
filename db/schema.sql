CREATE TYPE media_kind AS ENUM (
    'MOVIE',
    'SHOW',
    'SEASON',
    'EPISODE'
);

CREATE TABLE media (
    id INT NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    kind media_kind NOT NULL,
    UNIQUE (id, kind)
);

CREATE TABLE media_external_id (
    media_id INT NOT NULL PRIMARY KEY,
    trakt_id INT,
    trakt_slug TEXT,
    tvdb_id INT,
    imdb_id TEXT,
    tmdb_id INT,
    FOREIGN KEY (media_id) REFERENCES media (id)
);

CREATE TABLE movie (
    id INT NOT NULL PRIMARY KEY,
    kind media_kind NOT NULL GENERATED ALWAYS AS ('MOVIE'::media_kind) STORED,
    title TEXT NOT NULL,
    release_year INT,
    overview TEXT,
    tagline TEXT,
    runtime INT,
    FOREIGN KEY (id, kind) REFERENCES media (id, kind)
);

CREATE TABLE show (
    id INT NOT NULL PRIMARY KEY,
    kind media_kind NOT NULL GENERATED ALWAYS AS ('SHOW'::media_kind) STORED,
    title TEXT NOT NULL,
    release_year INT,
    overview TEXT,
    tagline TEXT,
    episode_runtime INT,
    FOREIGN KEY (id, kind) REFERENCES media (id, kind)
);

CREATE TABLE season (
    show_id INT NOT NULL,
    id INT NOT NULL PRIMARY KEY,
    kind media_kind NOT NULL GENERATED ALWAYS AS ('SEASON'::media_kind) STORED,
    title TEXT NOT NULL,
    number INT NOT NULL,
    overview TEXT,
    FOREIGN KEY (show_id) REFERENCES show (id),
    FOREIGN KEY (id, kind) REFERENCES media (id, kind),
    UNIQUE (show_id, number)
);

CREATE TABLE episode (
    show_id INT NOT NULL,
    season_id INT NOT NULL,
    id INT NOT NULL PRIMARY KEY,
    kind media_kind NOT NULL GENERATED ALWAYS AS ('EPISODE'::media_kind) STORED,
    title TEXT NOT NULL,
    number INT NOT NULL,
    overview TEXT,
    runtime INT,
    FOREIGN KEY (show_id) REFERENCES show (id),
    FOREIGN KEY (season_id) REFERENCES season (id),
    FOREIGN KEY (id, kind) REFERENCES media (id, kind),
    UNIQUE (show_id, season_id, number)
);

CREATE TABLE watch_history (
    id INT NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    media_id INT NOT NULL,
    media_kind media_kind NOT NULL,
    watched_at TIMESTAMPTZ NOT NULL,
    FOREIGN KEY (media_id, media_kind) REFERENCES media (id, kind),
    CONSTRAINT valid_media_kind 
        CHECK (media_kind IN ('MOVIE'::media_kind, 'EPISODE'::media_kind))
);

CREATE TYPE list_kind AS ENUM (
    'WATCHLIST',
    'FAVORITES',
    'PERSONAL'
);

CREATE TABLE list (
    id INT NOT NULL PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    kind list_kind NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- TODO: Insert this from the app?
INSERT INTO list (name, kind) VALUES
    ('Watchlist', 'WATCHLIST'::list_kind),
    ('Favorites', 'FAVORITES'::list_kind);

CREATE TABLE list_item (
    list_id INT NOT NULL,
    media_id INT NOT NULL,
    media_kind media_kind NOT NULL,
    listed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (list_id, media_id),
    FOREIGN KEY (list_id) REFERENCES list (id),
    FOREIGN KEY (media_id, media_kind) REFERENCES media (id, kind)
);