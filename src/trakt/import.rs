use deadpool_postgres::GenericClient;
use jiff::Timestamp;
use serde::Deserialize;

use crate::db::{
    Media, MediaExternalId, NewEpisode, NewMovie, NewSeason, NewShow, WatchHistory,
    get_media_by_trakt_id, get_season_by_show_and_number, insert_episode, insert_list_item,
    insert_movie, insert_season, insert_show, insert_watch_history,
};

#[derive(Deserialize, Debug, Clone)]
struct TraktExternalIds {
    trakt: i32,
    slug: Option<String>,
    tvdb: Option<i32>,
    imdb: Option<String>,
    tmdb: Option<i32>,
}

impl From<TraktExternalIds> for MediaExternalId {
    fn from(ids: TraktExternalIds) -> Self {
        MediaExternalId {
            trakt_id: Some(ids.trakt),
            trakt_slug: ids.slug,
            tvdb_id: ids.tvdb,
            imdb_id: ids.imdb,
            tmdb_id: ids.tmdb,
        }
    }
}

#[derive(Debug, Deserialize)]
struct TraktMovie {
    title: String,
    year: Option<i32>,
    ids: TraktExternalIds,
}

#[derive(Debug, Deserialize)]
struct TraktEpisode {
    #[serde(rename = "season")]
    season_number: i32,
    number: i32,
    title: String,
    ids: TraktExternalIds,
}

#[derive(Debug, Deserialize)]
struct TraktShow {
    title: String,
    year: Option<i32>,
    ids: TraktExternalIds,
}

#[derive(Debug, Deserialize)]
struct TraktSeason {
    number: i32,
    ids: TraktExternalIds,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum TraktMedia {
    #[serde(rename = "movie")]
    Movie { movie: TraktMovie },
    #[serde(rename = "episode")]
    Episode {
        episode: TraktEpisode,
        show: TraktShow,
    },
    #[serde(rename = "show")]
    Show { show: TraktShow },
    #[serde(rename = "season")]
    Season {
        season: TraktSeason,
        show: TraktShow,
    },
}

#[derive(Deserialize, Debug)]
struct WatchHistoryEntry {
    watched_at: Timestamp,
    #[serde(flatten)]
    media: TraktMedia,
    // unused fields:
    // action: String,
    // progress: f32,
    // duration: Option<i32>,
}

// TODO: error handling
pub async fn import_zip<C: GenericClient, R: std::io::Read + std::io::Seek>(
    conn: &mut C,
    zip_file: &mut R,
) -> anyhow::Result<()> {
    let mut zip = zip::ZipArchive::new(zip_file)?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let file_name = file.name().to_string();

        // TODO: use regex
        if file_name.contains("/watched/history-") && file_name.ends_with(".json") {
            import_watch_history(conn, &mut file).await?;
        }

        if file_name.ends_with("/lists/watchlist.json") {
            import_watchlist(conn, &mut file).await?;
        }
    }

    Ok(())
}

// TODO: error handling
// TODO: importing is quite slow right now. Caching shows and seasons would be nicer
pub async fn import_watch_history<C: GenericClient, R: std::io::Read>(
    conn: &mut C,
    history_file: &mut R,
) -> anyhow::Result<()> {
    let entries: Vec<WatchHistoryEntry> = serde_json::from_reader(history_file)?;

    for entry in entries {
        let media = match entry.media {
            TraktMedia::Episode {
                episode: trakt_episode,
                show: trakt_show,
            } => {
                let show = get_or_create_show(conn, &trakt_show).await?;
                let season = get_or_create_season(conn, &show, trakt_episode.season_number).await?;
                get_or_create_episode(conn, &show, &season, &trakt_episode).await?
            }
            TraktMedia::Movie { movie: trakt_movie } => {
                get_or_create_movie(conn, &trakt_movie).await?
            }
            _ => panic!("Unsupported media type in watch history: {:?}", entry.media),
        };

        insert_watch_history(
            conn,
            &WatchHistory {
                watched_at: entry.watched_at,
                media: media,
            },
        )
        .await?
    }

    Ok(())
}

#[derive(Deserialize, Debug)]
struct WatchlistEntry {
    listed_at: jiff::Timestamp,
    #[serde(flatten)]
    media: TraktMedia,
    // unused fields:
    // rank: i32,
    // notes: Option<String>,
}

pub async fn import_watchlist<C: GenericClient, R: std::io::Read>(
    conn: &mut C,
    watchlist_file: &mut R,
) -> anyhow::Result<()> {
    let entries: Vec<WatchlistEntry> = serde_json::from_reader(watchlist_file)?;

    for entry in entries {
        let media = match entry.media {
            TraktMedia::Episode {
                episode: trakt_episode,
                show: trakt_show,
            } => {
                let show = get_or_create_show(conn, &trakt_show).await?;
                let season = get_or_create_season(conn, &show, trakt_episode.season_number).await?;
                get_or_create_episode(conn, &show, &season, &trakt_episode).await?
            }
            TraktMedia::Movie { movie: trakt_movie } => {
                get_or_create_movie(conn, &trakt_movie).await?
            }
            TraktMedia::Show { show: trakt_show } => get_or_create_show(conn, &trakt_show).await?,
            TraktMedia::Season {
                season: trakt_season,
                show: trakt_show,
            } => {
                let show = get_or_create_show(conn, &trakt_show).await?;
                get_or_create_season(conn, &show, trakt_season.number).await?
            }
        };

        // TODO: remove hardcoded list id
        insert_list_item(conn, &1, &media, Some(&entry.listed_at)).await?;
    }

    Ok(())
}

async fn get_or_create_show<C: GenericClient>(
    conn: &mut C,
    trakt_show: &TraktShow,
) -> anyhow::Result<Media> {
    let media = match get_media_by_trakt_id(conn, trakt_show.ids.trakt).await? {
        Some(show) => show,
        None => {
            insert_show(
                conn,
                &NewShow {
                    title: trakt_show.title.clone(),
                    release_year: trakt_show.year,
                    overview: None,
                    tagline: None,
                    episode_runtime: None,
                    external_ids: Some(trakt_show.ids.clone().into()),
                    seasons: None,
                },
            )
            .await?
        }
    };

    Ok(media)
}

async fn get_or_create_season<C: GenericClient>(
    conn: &mut C,
    show: &Media,
    season_number: i32,
) -> anyhow::Result<Media> {
    let media = match get_season_by_show_and_number(conn, &show, season_number).await? {
        Some(season) => season,
        None => {
            insert_season(
                conn,
                &show,
                &NewSeason {
                    title: format!("Season {}", season_number),
                    number: season_number,
                    overview: None,
                    external_ids: None,
                    episodes: None,
                },
            )
            .await?
        }
    };

    Ok(media)
}

async fn get_or_create_episode<C: GenericClient>(
    conn: &mut C,
    show: &Media,
    season: &Media,
    trakt_episode: &TraktEpisode,
) -> anyhow::Result<Media> {
    let media = match get_media_by_trakt_id(conn, trakt_episode.ids.trakt).await? {
        Some(episode) => episode,
        None => {
            insert_episode(
                conn,
                &show,
                &season,
                &NewEpisode {
                    title: trakt_episode.title.clone(),
                    number: trakt_episode.number,
                    overview: None,
                    runtime: None,
                    external_ids: Some(trakt_episode.ids.clone().into()),
                },
            )
            .await?
        }
    };

    Ok(media)
}

async fn get_or_create_movie<C: GenericClient>(
    conn: &mut C,
    trakt_movie: &TraktMovie,
) -> anyhow::Result<Media> {
    let media = match get_media_by_trakt_id(conn, trakt_movie.ids.trakt).await? {
        Some(media) => media,
        None => {
            insert_movie(
                conn,
                &NewMovie {
                    title: trakt_movie.title.clone(),
                    release_year: trakt_movie.year,
                    external_ids: Some(trakt_movie.ids.clone().into()),
                    overview: None,
                    tagline: None,
                    runtime: None,
                },
            )
            .await?
        }
    };

    Ok(media)
}
