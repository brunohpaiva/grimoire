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

struct Episode {
    id: i32,
    title: String,
    number: i32,
    overview: Option<String>,
    play_count: i64,
}

#[derive(Template)]
#[template(path = "show_season.html")]
pub struct ShowSeasonTemplate {
    title: String,
    season_number: i32,
    show_id: i32,
    show_title: String,
    overview: Option<String>,
    total_episodes_count: i64,
    total_episodes_watched: i64,
    total_play_count: i64,
    episodes: Vec<Episode>,
}

#[derive(Deserialize)]
pub struct GetShowSeasonParams {
    show_id: i32,
    season_number: i32,
}

pub async fn get_show_season(
    State(state): State<Arc<AppState>>,
    Path(params): Path<GetShowSeasonParams>,
) -> Result<impl IntoResponse, AppError> {
    let conn = state
        .pool
        .get()
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let rows = conn
        .query(
            "
            SELECT se.id AS season_id, se.title AS season_title, se.overview, sh.title AS show_title,
            ep.id AS episode_id, ep.title AS episode_title, ep.number AS episode_number, 
            ep.overview AS episode_overview, COUNT(wh.watched_at) AS play_count,
            se.number AS season_number FROM season se
            INNER JOIN show sh ON sh.id = se.show_id
            INNER JOIN episode ep ON ep.season_id = se.id
            LEFT JOIN watch_history wh ON wh.media_id = ep.id AND wh.media_kind = 'EPISODE'
            WHERE sh.id = $1 AND se.number = $2
            GROUP BY se.id, sh.title, ep.id
            ORDER BY ep.number
            ",
            &[&params.show_id, &params.season_number],
        )
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    if rows.is_empty() {
        return Err(AppError::NotFound);
    }

    let mut template = ShowSeasonTemplate {
        title: String::new(),
        season_number: 0,
        show_id: params.show_id,
        show_title: String::new(),
        overview: None,
        total_episodes_count: 0,
        total_episodes_watched: 0,
        total_play_count: 0,
        episodes: Vec::new(),
    };

    for (row_idx, row) in rows.iter().enumerate() {
        if row_idx == 0 {
            template.title = row.get(1);
            template.overview = row.get(2);
            template.show_title = row.get(3);
            template.season_number = row.get(9);
        }

        let play_count: i64 = row.get(8);

        template.total_episodes_count += 1;
        template.total_play_count += play_count;

        if play_count > 0 {
            template.total_episodes_watched += 1;
        }

        template.episodes.push(Episode {
            id: row.get(4),
            title: row.get(5),
            number: row.get(6),
            overview: row.get(7),
            play_count,
        });
    }

    Ok(HtmlTemplate(template))
}
