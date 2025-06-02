use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    AppState,
    response::{AppError, HtmlTemplate},
};

#[derive(Template)]
#[template(path = "movie.html")]
pub struct MovieTemplate {
    id: i32,
    title: String,
    release_year: i32,
    play_count: i64,
    overview: Option<String>,
    tagline: Option<String>,
    runtime: Option<i32>,
}

pub async fn get_movie(
    State(state): State<Arc<AppState>>,
    Path(movie_id): Path<i32>,
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
            &[&movie_id],
        )
        .await
        .map_err(|err| AppError::Internal(err.into()))?
    else {
        return Err(AppError::NotFound);
    };

    Ok(HtmlTemplate(MovieTemplate {
        id: row.get(0),
        title: row.get(1),
        release_year: row.get(2),
        play_count: row.get(3),
        overview: row.get(4),
        tagline: row.get(5),
        runtime: row.get(6),
    }))
}
