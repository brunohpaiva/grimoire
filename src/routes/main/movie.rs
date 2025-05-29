use std::sync::Arc;

use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::AppState;

#[derive(Template, WebTemplate)]
#[template(path = "movie.html")]
pub struct MovieTemplate {
    id: i32,
    title: String,
    release_year: i32,
    play_count: i64,
}

pub async fn get_movie(
    State(state): State<Arc<AppState>>,
    Path(movie_id): Path<i32>,
) -> Result<MovieTemplate, Response> {
    let conn = state
        .pool
        .get()
        .await
        .inspect_err(|err| eprintln!("{:?}", err))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    let Some(row) = conn
        .query_opt(
            "
            SELECT mo.id, mo.title, mo.release_year, COUNT(wh.watched_at) AS play_count FROM movie mo
            LEFT JOIN watch_history wh ON mo.id = wh.media_id AND wh.media_kind = 'MOVIE'
            WHERE mo.id = $1
            GROUP BY mo.id
            ",
            &[&movie_id],
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    else {
        return Err(StatusCode::NOT_FOUND.into_response());
    };

    Ok(MovieTemplate {
        id: row.get(0),
        title: row.get(1),
        release_year: row.get(2),
        play_count: row.get(3),
    })
}
