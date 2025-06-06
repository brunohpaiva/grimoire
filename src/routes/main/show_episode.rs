use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    AppState,
    response::{AppError, HtmlTemplate},
};

#[derive(Template)]
#[template(path = "show_episode.html")]
pub struct ShowEpisodeTemplate {
    episode_id: i32,
    title: String,
    episode_number: i32,
    show_id: i32,
    show_title: String,
    season_title: String,
    season_number: i32,
    overview: Option<String>,
    play_count: i64,
}

#[derive(Deserialize)]
pub struct GetShowEpisodeParams {
    show_id: i32,
    season_number: i32,
    episode_number: i32,
}

pub async fn get_show_episode(
    State(state): State<Arc<AppState>>,
    Path(params): Path<GetShowEpisodeParams>,
) -> Result<impl IntoResponse, AppError> {
    let conn = state
        .pool
        .get()
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let Some(row) = conn
        .query_opt(
            "
            SELECT sh.id AS show_id, sh.title AS show_title, se.title AS season_title,
            se.number AS season_number, ep.id AS episode_id, 
            ep.title AS episode_title, ep.number AS episode_number, 
            ep.overview AS episode_overview, COUNT(wh.watched_at) AS play_count FROM episode ep
            INNER JOIN season se ON se.id = ep.season_id
            INNER JOIN show sh ON sh.id = ep.show_id
            LEFT JOIN watch_history wh ON wh.media_id = ep.id AND wh.media_kind = 'EPISODE'
            WHERE sh.id = $1 AND se.number = $2 AND ep.number = $3
            GROUP BY se.id, sh.id, ep.id
            ",
            &[
                &params.show_id,
                &params.season_number,
                &params.episode_number,
            ],
        )
        .await
        .map_err(|err| AppError::Internal(err.into()))?
    else {
        return Err(AppError::NotFound);
    };

    let template = ShowEpisodeTemplate {
        episode_id: row.get(4),
        title: row.get(5),
        episode_number: row.get(6),
        show_id: row.get(0),
        show_title: row.get(1),
        season_title: row.get(2),
        season_number: row.get(3),
        overview: row.get(7),
        play_count: row.get(8),
    };

    Ok(HtmlTemplate(template))
}
