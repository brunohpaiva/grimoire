use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;
use tracing::error;

use crate::{
    AppState,
    db::{
        Media, MediaExternalId, MediaKind, NewEpisode, NewMovie, NewSeason, NewShow,
        get_media_by_tmdb_id, insert_movie, insert_show,
    },
    response::AppError,
    tmdb::TmdbId,
};

#[derive(Deserialize)]
pub struct AddMediaParams {
    tmdb_id: TmdbId,
    tmdb_type: String,
}

pub async fn post_add_media(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AddMediaParams>,
) -> Result<Redirect, AppError> {
    let media_kind = match params.tmdb_type.as_str() {
        "movie" => MediaKind::Movie,
        "tv" => MediaKind::Show,
        _ => return Err(AppError::BadRequest),
    };

    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let media = get_media_by_tmdb_id(&conn, &params.tmdb_id, &media_kind)
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    if let Some(media) = media {
        return Ok(redirect_to(&media));
    }

    let media = match media_kind {
        MediaKind::Movie => {
            let full_movie = state
                .tmdb_api
                .fetch_full_movie(&params.tmdb_id)
                .await
                .map_err(|err| AppError::Internal(err.into()))?;

            insert_movie(
                &mut conn,
                &NewMovie {
                    title: full_movie.original_title,
                    release_year: full_movie
                        .release_date
                        .map(|date| date.year() as i32)
                        .unwrap_or_else(|| 0),
                    overview: Some(full_movie.overview),
                    tagline: Some(full_movie.tagline),
                    runtime: Some(full_movie.runtime),
                    external_ids: Some(MediaExternalId {
                        trakt_id: None,
                        trakt_slug: None,
                        tmdb_id: Some(full_movie.id.0),
                        imdb_id: Some(full_movie.imdb_id.to_string()),
                        tvdb_id: None,
                    }),
                },
            )
            .await
            .map_err(|err| AppError::Internal(err.into()))?
        }
        MediaKind::Show => {
            let full_show = state
                .tmdb_api
                .fetch_full_show(&params.tmdb_id)
                .await
                .map_err(|err| AppError::Internal(err.into()))?;

            let mut seasons: Vec<NewSeason> = vec![];

            for season in full_show.seasons.iter() {
                // FIXME: this should get rate limited by TMDB with 50+ seasons.
                let episodes = state
                    .tmdb_api
                    .fetch_full_season(&full_show.id, season.season_number)
                    .await
                    .inspect_err(|err| error!("{:?}", err))
                    .map_err(|err| AppError::Internal(err.into()))?
                    .episodes
                    .iter()
                    .map(|episode| NewEpisode {
                        title: episode.name.to_owned(),
                        number: episode.episode_number,
                        overview: Some(episode.overview.to_owned()),
                        runtime: episode.runtime,
                        external_ids: Some(MediaExternalId {
                            trakt_id: None,
                            trakt_slug: None,
                            tvdb_id: None,
                            imdb_id: None,
                            tmdb_id: Some(episode.id.0),
                        }),
                    })
                    .collect();

                seasons.push(NewSeason {
                    title: season.name.clone(),
                    number: season.season_number,
                    overview: Some(season.overview.clone()),
                    external_ids: Some(MediaExternalId {
                        trakt_id: None,
                        trakt_slug: None,
                        tvdb_id: None,
                        imdb_id: None,
                        tmdb_id: Some(season.id.0),
                    }),
                    episodes: Some(episodes),
                });
            }

            insert_show(
                &mut conn,
                &NewShow {
                    title: full_show.title,
                    release_year: full_show
                        .release_date
                        .map(|date| date.year() as i32)
                        .unwrap_or_else(|| 0),
                    overview: Some(full_show.overview),
                    tagline: Some(full_show.tagline),
                    episode_runtime: full_show.episode_runtimes.get(0).copied(),
                    seasons: Some(seasons),
                    external_ids: Some(MediaExternalId {
                        trakt_id: None,
                        trakt_slug: None,
                        tmdb_id: Some(full_show.id.0),
                        imdb_id: None,
                        tvdb_id: None,
                    }),
                },
            )
            .await
            .map_err(|err| AppError::Internal(err.into()))?
        }
        _ => unreachable!(),
    };

    Ok(redirect_to(&media))
}

fn redirect_to(media: &Media) -> Redirect {
    let base_uri = match media.kind {
        MediaKind::Movie => "/movie/",
        MediaKind::Show => "/show/",
        _ => unreachable!("media returned by tmdb id should be only Movie or Show"),
    };

    Redirect::to(format!("{}{}", base_uri, media.id).as_str())
}
