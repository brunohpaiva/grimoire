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
    release_year INT NOT NULL,
    FOREIGN KEY (id, kind) REFERENCES media (id, kind)
);

CREATE TABLE show (
    id INT NOT NULL PRIMARY KEY,
    kind media_kind NOT NULL GENERATED ALWAYS AS ('SHOW'::media_kind) STORED,
    title TEXT NOT NULL,
    release_year INT NOT NULL,
    FOREIGN KEY (id, kind) REFERENCES media (id, kind)
);

CREATE TABLE season (
    show_id INT NOT NULL,
    id INT NOT NULL PRIMARY KEY,
    kind media_kind NOT NULL GENERATED ALWAYS AS ('SEASON'::media_kind) STORED,
    title TEXT NOT NULL,
    FOREIGN KEY (show_id) REFERENCES show (id),
    FOREIGN KEY (id, kind) REFERENCES media (id, kind)
);

CREATE TABLE episode (
    show_id INT NOT NULL,
    season_id INT NOT NULL,
    id INT NOT NULL PRIMARY KEY,
    kind media_kind NOT NULL GENERATED ALWAYS AS ('EPISODE'::media_kind) STORED,
    title TEXT NOT NULL,
    FOREIGN KEY (show_id) REFERENCES show (id),
    FOREIGN KEY (season_id) REFERENCES season (id),
    FOREIGN KEY (id, kind) REFERENCES media (id, kind)
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