use deadpool_postgres::{GenericClient, tokio_postgres};
use postgres_types::{FromSql, ToSql};
use thiserror::Error;

#[derive(Debug, ToSql, FromSql)]
#[postgres(name = "media_kind", rename_all = "UPPERCASE")]
pub enum MediaKind {
    Movie,
    Show,
    Season,
    Episode,
}

#[derive(Debug)]
pub struct Media {
    id: i32,
    kind: MediaKind,
}

#[derive(Debug, Error)]
pub enum GetMediaIdError {
    #[error("failed to query media id")]
    Query(#[source] tokio_postgres::Error),
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
    .map_err(GetMediaIdError::Query)
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
pub enum InsertMovieError {
    #[error("failed to insert media")]
    InsertMedia(#[source] InsertMediaError),
    #[error("failed to insert movie")]
    Insert(#[source] tokio_postgres::Error),
    #[error("failed to start transaction")]
    StartTransaction(#[source] tokio_postgres::Error),
    #[error("failed to query if movie already exists")]
    QueryExisting(#[source] GetMediaIdError),
    #[error("failed to commit transaction")]
    CommitTransaction(#[source] tokio_postgres::Error),
}

pub struct NewMovie {
    pub title: String,
    pub release_year: i32,
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

    tx.execute(
        "INSERT INTO movie (id, title, release_year) VALUES ($1, $2, $3)",
        &[&media.id, &new_movie.title, &new_movie.release_year],
    )
    .await
    .map_err(InsertMovieError::Insert)?;

    tx.commit()
        .await
        .map_err(InsertMovieError::CommitTransaction)?;

    Ok(Media {
        id: media.id,
        kind: MediaKind::Movie,
    })
}

#[derive(Debug, Error)]
#[error("failed to insert watch history")]
pub struct InsertWatchHistoryError(#[source] tokio_postgres::Error);

pub struct WatchHistory {
    pub watched_at: jiff::Timestamp,
    pub media: Media,
}

pub async fn insert_watch_history<C: GenericClient>(
    conn: &mut C,
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
