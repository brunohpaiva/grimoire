use std::sync::Arc;

use crate::filters;
use askama::Template;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    AppState,
    db::{WatchHistoryEntry, WatchHistoryEntryMedia, get_watch_history},
    response::{AppError, HtmlTemplate},
};

pub struct ListItem {

}

#[derive(Template)]
#[template(path = "list.html")]
pub struct ListTemplate {
    id: i32,
    title: String,
    description: Option<String>,
    items: Vec<ListItem>,
}

pub async fn get_list(
    State(state): State<Arc<AppState>>,
    Path(list_id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let conn = state
        .pool
        .get()
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let Some(row) = conn
        .query_opt(
            "
            SELECT mo.id, mo.title, mo.release_year, COUNT(wh.watched_at) AS play_count,
            mo.overview, mo.tagline, mo.runtime FROM movie mo
            LEFT JOIN watch_history wh ON mo.id = wh.media_id AND wh.media_kind = 'MOVIE'
            WHERE mo.id = $1
            GROUP BY mo.id
            ",
            &[&list_id],
        )
        .await
        .map_err(|err| AppError::Internal(err.into()))?
    else {
        return Err(AppError::NotFound);
    };

    // TODO: implement pagination...
    let movie_history = get_watch_history(
        &conn,
        999,
        Some(crate::db::GetWatchHistoryFilter::Movie(movie_id)),
    )
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

    Ok(HtmlTemplate(MovieTemplate {
        id: row.get(0),
        title: row.get(1),
        release_year: row.get(2),
        play_count: row.get(3),
        overview: row.get(4),
        tagline: row.get(5),
        runtime: row.get(6),
        history: movie_history,
    }))
}
