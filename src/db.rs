use deadpool_postgres::{Config, GenericClient, Pool, Runtime, tokio_postgres};
use postgres_types::{FromSql, ToSql};
use serde::Deserialize;
use thiserror::Error;
use tokio_postgres::NoTls;

use crate::{config::AppConfig, tmdb::TmdbId};

pub fn create_pool(config: &AppConfig) -> Result<Pool, deadpool_postgres::CreatePoolError> {
    let mut cfg = Config::new();
    cfg.host = Some(config.db_host.clone());
    cfg.port = Some(config.db_port.clone());
    cfg.dbname = Some(config.db_name.clone());
    cfg.user = Some(config.db_user.clone());
    cfg.password = Some(config.db_password.clone());

    Ok(cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
}

#[derive(Debug, Deserialize, ToSql, FromSql)]
#[postgres(name = "media_kind", rename_all = "UPPERCASE")]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Movie,
    Show,
    Season,
    Episode,
}

#[derive(Debug)]
pub struct Media {
    pub id: i32,
    pub kind: MediaKind,
}

#[derive(Debug)]
pub struct MediaExternalId {
    pub trakt_id: Option<i32>,
    pub trakt_slug: Option<String>,
    pub tvdb_id: Option<i32>,
    pub imdb_id: Option<String>,
    pub tmdb_id: Option<i32>,
}

#[derive(Debug, Error)]
#[error("failed to query media id")]
pub struct GetMediaIdError(#[source] tokio_postgres::Error);

pub async fn get_media_by_id<C: GenericClient>(
    conn: &C,
    id: i32,
    media_kind: Option<MediaKind>,
) -> Result<Option<Media>, GetMediaIdError> {
    // TODO: simplify this
    if let Some(media_kind) = media_kind {
        conn.query_opt(
            "SELECT m.id, m.kind FROM media m
            WHERE m.id = $1 AND m.kind = $2",
            &[&id, &media_kind],
        )
        .await
        .map_err(GetMediaIdError)
        .map(|opt_row| {
            opt_row.map(|row| Media {
                id: row.get(0),
                kind: row.get(1),
            })
        })
    } else {
        conn.query_opt("SELECT m.id, m.kind FROM media m WHERE m.id = $1", &[&id])
            .await
            .map_err(GetMediaIdError)
            .map(|opt_row| {
                opt_row.map(|row| Media {
                    id: row.get(0),
                    kind: row.get(1),
                })
            })
    }
}

pub async fn get_media_by_tmdb_id<C: GenericClient>(
    conn: &C,
    tmdb_id: &TmdbId,
    media_kind: &MediaKind,
) -> Result<Option<Media>, GetMediaIdError> {
    conn.query_opt(
        "SELECT m.id, m.kind FROM media_external_id mei 
            INNER JOIN media m ON mei.media_id = m.id
            WHERE mei.tmdb_id = $1 AND m.kind = $2
            ",
        &[&tmdb_id.0, &media_kind],
    )
    .await
    .map_err(GetMediaIdError)
    .map(|opt_row| {
        opt_row.map(|row| Media {
            id: row.get(0),
            kind: row.get(1),
        })
    })
}

pub async fn get_media_by_trakt_id<C: GenericClient>(
    conn: &C,
    trakt_id: i32,
) -> Result<Option<Media>, GetMediaIdError> {
    conn.query_opt(
        "SELECT m.id, m.kind FROM media_external_id mei 
            INNER JOIN media m ON mei.media_id = m.id
            WHERE mei.trakt_id = $1
            ",
        &[&trakt_id],
    )
    .await
    .map_err(GetMediaIdError)
    .map(|opt_row| {
        opt_row.map(|row| Media {
            id: row.get(0),
            kind: row.get(1),
        })
    })
}

#[derive(Debug, Error)]
#[error("failed to insert media")]
pub struct InsertMediaError(#[source] tokio_postgres::Error);

pub async fn insert_media<C: GenericClient>(
    conn: &C,
    kind: MediaKind,
) -> Result<Media, InsertMediaError> {
    let media = conn
        .query_one(
            "INSERT INTO media (kind) VALUES ($1) RETURNING id",
            &[&kind],
        )
        .await
        .map_err(InsertMediaError)
        .map(|row| Media {
            id: row.get(0),
            kind,
        })?;

    Ok(media)
}

#[derive(Debug, Error)]
#[error("failed to insert media external id")]
pub struct InsertMediaExternalIdError(#[source] tokio_postgres::Error);

pub async fn insert_media_external_id<C: GenericClient>(
    conn: &C,
    media: &Media,
    external_id: &MediaExternalId,
) -> Result<(), InsertMediaExternalIdError> {
    conn.execute(
        "INSERT INTO media_external_id (media_id, trakt_id, trakt_slug, tvdb_id, imdb_id, tmdb_id) 
        VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &media.id,
            &external_id.trakt_id,
            &external_id.trakt_slug,
            &external_id.tvdb_id,
            &external_id.imdb_id,
            &external_id.tmdb_id,
        ],
    )
    .await
    .map_err(InsertMediaExternalIdError)?;

    Ok(())
}

#[derive(Debug, Error)]
pub enum InsertMovieError {
    #[error("failed to insert media")]
    InsertMedia(#[source] InsertMediaError),
    #[error("failed to insert media external id")]
    InsertMediaExternalId(#[source] InsertMediaExternalIdError),
    #[error("failed to insert movie")]
    InsertMovie(#[source] tokio_postgres::Error),
    #[error("failed to start transaction")]
    StartTransaction(#[source] tokio_postgres::Error),
    #[error("failed to commit transaction")]
    CommitTransaction(#[source] tokio_postgres::Error),
}

pub struct NewMovie {
    pub title: String,
    pub release_year: i32,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub runtime: Option<i32>,
    pub external_ids: Option<MediaExternalId>,
}

pub async fn insert_movie<C: GenericClient>(
    conn: &mut C,
    new_movie: &NewMovie,
) -> Result<Media, InsertMovieError> {
    let tx = conn
        .transaction()
        .await
        .map_err(InsertMovieError::StartTransaction)?;

    let media = insert_media(&tx, MediaKind::Movie)
        .await
        .map_err(InsertMovieError::InsertMedia)?;

    if let Some(external_ids) = &new_movie.external_ids {
        insert_media_external_id(&tx, &media, external_ids)
            .await
            .map_err(InsertMovieError::InsertMediaExternalId)?;
    }

    tx.execute(
        "INSERT INTO movie (id, title, release_year, overview, tagline, runtime) VALUES ($1, $2, $3, $4, $5, $6)",
        &[
            &media.id,
            &new_movie.title,
            &new_movie.release_year,
            &new_movie.overview,
            &new_movie.tagline,
            &new_movie.runtime
        ],
    )
    .await
    .map_err(InsertMovieError::InsertMovie)?;

    tx.commit()
        .await
        .map_err(InsertMovieError::CommitTransaction)?;

    Ok(Media {
        id: media.id,
        kind: MediaKind::Movie,
    })
}

#[derive(Debug, Error)]
pub enum InsertShowError {
    #[error("failed to insert media")]
    InsertMedia(#[source] InsertMediaError),
    #[error("failed to insert media external id")]
    InsertMediaExternalId(#[source] InsertMediaExternalIdError),
    #[error("failed to insert show")]
    InsertShow(#[source] tokio_postgres::Error),
    #[error("failed to insert season")]
    InsertSeason(#[source] InsertSeasonError),
    #[error("failed to start transaction")]
    StartTransaction(#[source] tokio_postgres::Error),
    #[error("failed to commit transaction")]
    CommitTransaction(#[source] tokio_postgres::Error),
}

pub struct NewShow {
    pub title: String,
    pub release_year: i32,
    pub overview: Option<String>,
    pub tagline: Option<String>,
    pub episode_runtime: Option<i32>,
    pub external_ids: Option<MediaExternalId>,
    pub seasons: Option<Vec<NewSeason>>,
}

pub async fn insert_show<C: GenericClient>(
    conn: &mut C,
    new_show: &NewShow,
) -> Result<Media, InsertShowError> {
    let mut tx = conn
        .transaction()
        .await
        .map_err(InsertShowError::StartTransaction)?;

    let media = insert_media(&tx, MediaKind::Show)
        .await
        .map_err(InsertShowError::InsertMedia)?;

    if let Some(external_ids) = &new_show.external_ids {
        insert_media_external_id(&tx, &media, external_ids)
            .await
            .map_err(InsertShowError::InsertMediaExternalId)?;
    }

    tx.execute(
        "INSERT INTO show (id, title, release_year, overview, tagline, episode_runtime) VALUES ($1, $2, $3, $4, $5, $6)",
        &[&media.id, &new_show.title, &new_show.release_year, &new_show.overview, &new_show.tagline, &new_show.episode_runtime],
    )
    .await
    .map_err(InsertShowError::InsertShow)?;

    if let Some(seasons) = &new_show.seasons {
        for season in seasons {
            insert_season(&mut tx, &media, season)
                .await
                .map_err(InsertShowError::InsertSeason)?;
        }
    }

    tx.commit()
        .await
        .map_err(InsertShowError::CommitTransaction)?;

    Ok(media)
}

pub async fn get_season_by_show_and_number<C: GenericClient>(
    conn: &C,
    show: &Media,
    number: i32,
) -> Result<Option<Media>, GetMediaIdError> {
    conn.query_opt(
        "SELECT s.id, s.kind FROM season s WHERE s.show_id = $1 AND s.number = $2",
        &[&show.id, &number],
    )
    .await
    .map_err(GetMediaIdError)
    .map(|opt_row| {
        opt_row.map(|row| Media {
            id: row.get(0),
            kind: row.get(1),
        })
    })
}

#[derive(Debug, Error)]
pub enum InsertSeasonError {
    #[error("failed to insert media")]
    InsertMedia(#[source] InsertMediaError),
    #[error("failed to insert media external id")]
    InsertMediaExternalId(#[source] InsertMediaExternalIdError),
    #[error("failed to insert season")]
    InsertSeason(#[source] tokio_postgres::Error),
    #[error("failed to start transaction")]
    StartTransaction(#[source] tokio_postgres::Error),
    #[error("failed to commit transaction")]
    CommitTransaction(#[source] tokio_postgres::Error),
}

pub struct NewSeason {
    pub title: String,
    pub number: i32,
    pub overview: Option<String>,
    pub external_ids: Option<MediaExternalId>,
}

pub async fn insert_season<C: GenericClient>(
    conn: &mut C,
    show: &Media,
    new_season: &NewSeason,
) -> Result<Media, InsertSeasonError> {
    let tx = conn
        .transaction()
        .await
        .map_err(InsertSeasonError::StartTransaction)?;

    let media = insert_media(&tx, MediaKind::Season)
        .await
        .map_err(InsertSeasonError::InsertMedia)?;

    if let Some(external_ids) = &new_season.external_ids {
        insert_media_external_id(&tx, &media, external_ids)
            .await
            .map_err(InsertSeasonError::InsertMediaExternalId)?;
    }

    tx.execute(
        "INSERT INTO season (show_id, id, title, number, overview) VALUES ($1, $2, $3, $4, $5)",
        &[
            &show.id,
            &media.id,
            &new_season.title,
            &new_season.number,
            &new_season.overview,
        ],
    )
    .await
    .map_err(InsertSeasonError::InsertSeason)?;

    tx.commit()
        .await
        .map_err(InsertSeasonError::CommitTransaction)?;

    Ok(media)
}

#[derive(Debug, Error)]
pub enum InsertEpisodeError {
    #[error("failed to insert media")]
    InsertMedia(#[source] InsertMediaError),
    #[error("failed to insert media external id")]
    InsertMediaExternalId(#[source] InsertMediaExternalIdError),
    #[error("failed to insert episode")]
    InsertEpisode(#[source] tokio_postgres::Error),
    #[error("failed to start transaction")]
    StartTransaction(#[source] tokio_postgres::Error),
    #[error("failed to commit transaction")]
    CommitTransaction(#[source] tokio_postgres::Error),
}

pub struct NewEpisode {
    pub title: String,
    pub number: i32,
    pub external_ids: Option<MediaExternalId>,
}

pub async fn insert_episode<C: GenericClient>(
    conn: &mut C,
    show: &Media,
    season: &Media,
    new_episode: &NewEpisode,
) -> Result<Media, InsertEpisodeError> {
    let tx = conn
        .transaction()
        .await
        .map_err(InsertEpisodeError::StartTransaction)?;

    let media = insert_media(&tx, MediaKind::Episode)
        .await
        .map_err(InsertEpisodeError::InsertMedia)?;

    if let Some(external_ids) = &new_episode.external_ids {
        insert_media_external_id(&tx, &media, external_ids)
            .await
            .map_err(InsertEpisodeError::InsertMediaExternalId)?;
    }

    tx.execute(
        "INSERT INTO episode (show_id, season_id, id, title, number) VALUES ($1, $2, $3, $4, $5)",
        &[
            &show.id,
            &season.id,
            &media.id,
            &new_episode.title,
            &new_episode.number,
        ],
    )
    .await
    .map_err(InsertEpisodeError::InsertEpisode)?;

    tx.commit()
        .await
        .map_err(InsertEpisodeError::CommitTransaction)?;

    Ok(media)
}

#[derive(Debug, Error)]
#[error("failed to insert watch history")]
pub struct InsertWatchHistoryError(#[source] tokio_postgres::Error);

pub struct WatchHistory {
    pub watched_at: jiff::Timestamp,
    pub media: Media,
}

pub async fn insert_watch_history<C: GenericClient>(
    conn: &C,
    watch_history: &WatchHistory,
) -> Result<(), InsertWatchHistoryError> {
    conn.execute(
        "INSERT INTO watch_history (media_id, media_kind, watched_at) VALUES ($1, $2, $3)",
        &[
            &watch_history.media.id,
            &watch_history.media.kind,
            &watch_history.watched_at,
        ],
    )
    .await
    .map_err(InsertWatchHistoryError)?;

    Ok(())
}
