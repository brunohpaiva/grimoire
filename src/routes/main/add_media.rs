use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::{
    AppState,
    db::{
        Media, MediaExternalId, MediaKind, NewMovie, NewSeason, NewShow, get_media_by_tmdb_id,
        insert_movie, insert_show,
    },
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
) -> Result<Redirect, Response> {
    let media_kind = match params.tmdb_type.as_str() {
        "movie" => MediaKind::Movie,
        "tv" => MediaKind::Show,
        _ => return Err(StatusCode::BAD_REQUEST.into_response()),
    };

    let mut conn = state
        .pool
        .get()
        .await
        .inspect_err(|err| eprintln!("{:?}", err))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    let media = get_media_by_tmdb_id(&conn, &params.tmdb_id, &media_kind)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    if let Some(media) = media {
        return Ok(redirect_to(&media));
    }

    let media = match media_kind {
        MediaKind::Movie => {
            let full_movie = state
                .tmdb_api
                .fetch_full_movie(&params.tmdb_id)
                .await
                .inspect_err(|err| eprintln!("{:?}", err))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

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
            .inspect_err(|err| eprintln!("{:?}", err))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
        }
        MediaKind::Show => {
            let full_show = state
                .tmdb_api
                .fetch_full_show(&params.tmdb_id)
                .await
                .inspect_err(|err| eprintln!("{:?}", err))
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

            let seasons = full_show
                .seasons
                .iter()
                .map(|season| NewSeason {
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
                })
                .collect();

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
            .inspect_err(|err| eprintln!("{:?}", err))
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
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
