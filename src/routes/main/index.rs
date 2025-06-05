use std::sync::Arc;

use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::{
    AppState,
    db::{WatchHistoryEntryMedia, get_watch_history},
    filters,
    response::{AppError, HtmlTemplate},
};

struct RecentlyWatchedEntry {
    watched_at: jiff::Timestamp,
    url: String,
    media: WatchHistoryEntryMedia,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    recently_watched: Vec<RecentlyWatchedEntry>,
}

pub async fn get_index(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let conn = state
        .pool
        .get()
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let recently_watched = get_watch_history(&conn, 10, None)
        .await
        .inspect_err(|err| eprintln!("{:?}", err))
        .map_err(|err| AppError::Internal(err.into()))?
        .iter()
        .map(|entry| RecentlyWatchedEntry {
            watched_at: entry.watched_at,
            media: entry.media.to_owned(),
            url: match entry.media {
                WatchHistoryEntryMedia::Movie { id, .. } => format!("/movie/{}", id),
                WatchHistoryEntryMedia::Episode { show_id, .. } => format!("/show/{}", show_id),
            },
        })
        .collect();

    Ok(HtmlTemplate(IndexTemplate { recently_watched }))
}
