use std::sync::Arc;

use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::{
    AppState,
    db::MediaKind,
    response::{AppError, HtmlTemplate},
};

struct RecentlyWatchedEntry {
    // watched_at: jiff::Timestamp,
    url: String,
    title: String,
    season_title: Option<String>,
    show_title: Option<String>,
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

    let rows = conn
        .query(
            "
            SELECT wh.media_id, wh.media_kind, 
            COALESCE(ep.title, mo.title) AS title,
            se.title AS season_title,
            sh.title AS show_title, sh.id AS show_id FROM watch_history wh
            LEFT JOIN movie mo ON wh.media_id = mo.id AND wh.media_kind = 'MOVIE'
            LEFT JOIN episode ep ON wh.media_id = ep.id AND wh.media_kind = 'EPISODE'
            LEFT JOIN season se ON ep.season_id = se.id AND ep.show_id = se.show_id
            LEFT JOIN show sh ON ep.show_id = sh.id
            ORDER BY wh.watched_at DESC
            LIMIT 10
            ",
            &[],
        )
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let recently_watched = rows
        .iter()
        .map(|row| {
            let media_kind: MediaKind = row.get(1);

            RecentlyWatchedEntry {
                url: match media_kind {
                    MediaKind::Movie => {
                        format!("/movie/{}", row.get::<_, i32>(0))
                    }
                    MediaKind::Episode => {
                        // TODO: page for specific episodes...
                        let show_id = row
                            .get::<_, Option<i32>>(5)
                            .expect("show_id null in watch_history row");
                        format!("/show/{}", show_id)
                    }
                    _ => unreachable!("invalid media_kind in watch_history table"),
                },
                title: row.get(2),
                season_title: row.get(3),
                show_title: row.get(4),
            }
        })
        .collect();

    Ok(HtmlTemplate(IndexTemplate { recently_watched }))
}
