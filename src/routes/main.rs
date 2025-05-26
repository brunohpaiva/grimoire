use std::sync::Arc;

use askama::Template;
use askama_web::WebTemplate;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};

use crate::{AppState, db::MediaKind};

pub fn build_router() -> Router<Arc<AppState>> {
    Router::new().route("/", get(get_index))
}

async fn get_index(State(state): State<Arc<AppState>>) -> Result<IndexTemplate, Response> {
    let conn = state
        .pool
        .get()
        .await
        .inspect_err(|err| eprintln!("{:?}", err))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    let rows = conn
        .query(
            "
            SELECT wh.watched_at, wh.media_kind, 
            COALESCE(ep.title, mo.title) AS title,
            se.title AS season_title,
            sh.title AS show_title FROM watch_history wh
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    let recently_watched = rows
        .iter()
        .map(|row| RecentlyWatchedEntry {
            watched_at: row.get(0),
            media_kind: row.get(1),
            title: row.get(2),
            season_title: row.get(3),
            show_title: row.get(4),
        })
        .collect();

    Ok(IndexTemplate { recently_watched })
}

struct RecentlyWatchedEntry {
    watched_at: jiff::Timestamp,
    media_kind: MediaKind,
    title: String,
    season_title: Option<String>,
    show_title: Option<String>,
}

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
struct IndexTemplate {
    recently_watched: Vec<RecentlyWatchedEntry>,
}
