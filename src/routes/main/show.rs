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

struct Season {
    title: String,
    number: i32,
}

#[derive(Template)]
#[template(path = "show.html")]
pub struct ShowTemplate {
    id: i32,
    title: String,
    release_year: i32,
    overview: Option<String>,
    tagline: Option<String>,
    episode_runtime: Option<i32>,
    seasons: Vec<Season>,
}

pub async fn get_show(
    State(state): State<Arc<AppState>>,
    Path(show_id): Path<i32>,
) -> Result<impl IntoResponse, AppError> {
    let conn = state
        .pool
        .get()
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let rows = conn
        .query(
            "
            SELECT sh.id AS show_id, sh.title AS show_title, sh.release_year, 
            sh.overview AS show_overview, sh.tagline AS show_tagline,
            sh.episode_runtime, se.id AS season_id, se.title AS season_title, 
            se.number AS season_number, COUNT(wh.watched_at) AS play_count FROM show sh
            LEFT JOIN season se ON se.show_id = sh.id
            LEFT JOIN episode ep ON ep.season_id = se.id
            LEFT JOIN watch_history wh ON wh.media_id = ep.id AND wh.media_kind = 'EPISODE'
            WHERE sh.id = $1
            GROUP BY sh.id, se.id
            ORDER BY se.number
            ",
            &[&show_id],
        )
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    if rows.is_empty() {
        return Err(AppError::NotFound);
    }

    let mut template = ShowTemplate {
        id: 0,
        title: String::new(),
        release_year: 0,
        overview: None,
        tagline: None,
        episode_runtime: None,
        seasons: vec![],
    };

    for (row_idx, row) in rows.iter().enumerate() {
        if row_idx == 0 {
            template.id = row.get(0);
            template.title = row.get(1);
            template.release_year = row.get(2);
            template.overview = row.get(3);
            template.tagline = row.get(4);
            template.episode_runtime = row.get(5);
        }

        let Some(season_number) = row.get::<_, Option<i32>>(8) else {
            continue;
        };

        // TODO: add play count to season
        template.seasons.push(Season {
            title: row.get(7),
            number: season_number,
        });
    }

    Ok(HtmlTemplate(template))
}
