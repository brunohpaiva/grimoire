use std::sync::Arc;

use axum::{
    Router,
    response::IntoResponse,
    routing::{get, post},
};

use crate::{AppState, response::AppError};

mod add_media;
mod add_watch;
mod index;
mod movie;
mod search;
mod show;
mod show_episode;
mod show_season;
mod list;

pub fn build_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(index::get_index))
        // TODO: accept slugs for movie and show
        .route("/movie/{movie_id}", get(movie::get_movie))
        .route("/show/{show_id}", get(show::get_show))
        .route(
            "/show/{show_id}/season/{season_number}",
            get(show_season::get_show_season),
        )
        .route(
            "/show/{show_id}/season/{season_number}/episode/{episode_number}",
            get(show_episode::get_show_episode),
        )
        .route("/add-watch", post(add_watch::post_add_watch))
        .route("/search", get(search::get_search))
        .route("/add-media", post(add_media::post_add_media))
        .route("/list/{list_id}", get(list::get_list))
        .fallback(fallback_handler)
}

async fn fallback_handler() -> impl IntoResponse {
    AppError::NotFound
}
