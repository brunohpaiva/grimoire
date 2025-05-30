use deadpool_postgres::GenericClient;
use jiff::Timestamp;
use serde::Deserialize;

use crate::db::{
    MediaExternalId, NewEpisode, NewMovie, NewSeason, NewShow, WatchHistory, get_media_by_trakt_id,
    get_season_by_show_and_number, insert_episode, insert_movie, insert_season, insert_show,
    insert_watch_history,
};

#[derive(Deserialize, Debug)]
struct ExternalIds {
    trakt: i32,
    slug: Option<String>,
    tvdb: Option<i32>,
    imdb: Option<String>,
    tmdb: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct Movie {
    title: String,
    year: i32,
    ids: ExternalIds,
}

#[derive(Debug, Deserialize)]
struct Episode {
    #[serde(rename = "season")]
    season_number: i32,
    number: i32,
    title: String,
    ids: ExternalIds,
}

#[derive(Debug, Deserialize)]
struct Show {
    title: String,
    year: i32,
    ids: ExternalIds,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum Media {
    #[serde(rename = "movie")]
    Movie { movie: Movie },
    #[serde(rename = "episode")]
    Episode { episode: Episode, show: Show },
}

#[derive(Deserialize, Debug)]
struct WatchHistoryEntry {
    watched_at: Timestamp,
    #[serde(flatten)]
    media: Media,
    // unused fields:
    // action: String,
    // progress: f32,
    // duration: Option<i32>,
}

// TODO: error handling
pub async fn import_zip<C: GenericClient, R: std::io::Read + std::io::Seek>(
    conn: &mut C,
    file: &mut R,
) -> anyhow::Result<()> {
    let mut zip = zip::ZipArchive::new(file)?;

    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;

        // TODO: use regex
        if !file.name().contains("/watched/history-") || !file.name().ends_with(".json") {
            continue;
        }

        import(conn, &mut file).await?;
    }

    Ok(())
}

// TODO: error handling
// TODO: importing is quite slow right now. Caching shows and seasons would be nicer
pub async fn import<C: GenericClient, R: std::io::Read>(
    conn: &mut C,
    history_file: &mut R,
) -> anyhow::Result<()> {
    let entries: Vec<WatchHistoryEntry> = serde_json::from_reader(history_file)?;

    for entry in entries {
        match entry.media {
            Media::Episode {
                episode: trakt_episode,
                show: trakt_show,
            } => {
                // TODO: this is ugly. extract to a separate function
                let show_external_ids = MediaExternalId {
                    trakt_id: Some(trakt_show.ids.trakt),
                    trakt_slug: trakt_show.ids.slug,
                    tvdb_id: trakt_show.ids.tvdb,
                    imdb_id: trakt_show.ids.imdb,
                    tmdb_id: trakt_show.ids.tmdb,
                };

                let show = match get_media_by_trakt_id(conn, trakt_show.ids.trakt).await? {
                    Some(show) => show,
                    None => {
                        insert_show(
                            conn,
                            &NewShow {
                                title: trakt_show.title,
                                release_year: trakt_show.year,
                                overview: None,
                                tagline: None,
                                episode_runtime: None,
                                external_ids: Some(show_external_ids),
                                seasons: None,
                            },
                        )
                        .await?
                    }
                };

                let season =
                    match get_season_by_show_and_number(conn, &show, trakt_episode.season_number)
                        .await?
                    {
                        Some(season) => season,
                        None => {
                            insert_season(
                                conn,
                                &show,
                                &NewSeason {
                                    title: format!("Season {}", trakt_episode.season_number),
                                    number: trakt_episode.season_number,
                                    overview: None,
                                    external_ids: None,
                                },
                            )
                            .await?
                        }
                    };

                let episode_external_ids = MediaExternalId {
                    trakt_id: Some(trakt_episode.ids.trakt),
                    trakt_slug: trakt_episode.ids.slug,
                    tvdb_id: trakt_episode.ids.tvdb,
                    imdb_id: trakt_episode.ids.imdb,
                    tmdb_id: trakt_episode.ids.tmdb,
                };

                let episode = match get_media_by_trakt_id(conn, trakt_episode.ids.trakt).await? {
                    Some(episode) => episode,
                    None => {
                        insert_episode(
                            conn,
                            &show,
                            &season,
                            &NewEpisode {
                                title: trakt_episode.title,
                                number: trakt_episode.number,
                                external_ids: Some(episode_external_ids),
                            },
                        )
                        .await?
                    }
                };

                insert_watch_history(
                    conn,
                    &WatchHistory {
                        watched_at: entry.watched_at,
                        media: episode,
                    },
                )
                .await?
            }
            Media::Movie { movie: trakt_movie } => {
                let external_ids = MediaExternalId {
                    trakt_id: Some(trakt_movie.ids.trakt),
                    trakt_slug: trakt_movie.ids.slug,
                    tvdb_id: trakt_movie.ids.tvdb,
                    imdb_id: trakt_movie.ids.imdb,
                    tmdb_id: trakt_movie.ids.tmdb,
                };

                let media = match get_media_by_trakt_id(conn, trakt_movie.ids.trakt).await? {
                    Some(media) => media,
                    None => {
                        insert_movie(
                            conn,
                            &NewMovie {
                                title: trakt_movie.title,
                                release_year: trakt_movie.year,
                                external_ids: Some(external_ids),
                                overview: None,
                                tagline: None,
                                runtime: None,
                            },
                        )
                        .await?
                    }
                };

                insert_watch_history(
                    conn,
                    &WatchHistory {
                        watched_at: entry.watched_at,
                        media,
                    },
                )
                .await?
            }
        };
    }

    Ok(())
}
