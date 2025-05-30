use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use crate::AppState;

mod add_media;
mod add_watch;
mod index;
mod movie;
mod search;
mod show;

pub fn build_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(index::get_index))
        // TODO: accept slugs for movie and show
        .route("/movie/{movie_id}", get(movie::get_movie))
        .route("/show/{show_id}", get(show::get_show))
        .route("/add-watch", post(add_watch::post_add_watch))
        .route("/search", get(search::get_search))
        .route("/add-media", post(add_media::post_add_media))
}
