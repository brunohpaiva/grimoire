use std::sync::Arc;

use askama::Template;
use axum::{extract::State, response::IntoResponse};

use crate::{
    AppState,
    db::MediaKind,
    response::{AppError, HtmlTemplate},
    filters,
};

enum RecentlyWatchedEntryMedia {
    Movie {
        title: String,
    },
    Episode {
        title: String,
        number: i32,
        season_number: i32,
        show_title: String,
    },
}

struct RecentlyWatchedEntry {
    watched_at: jiff::Timestamp,
    url: String,
    media: RecentlyWatchedEntryMedia,
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
            SELECT wh.watched_at, wh.media_kind, wh.media_id,
            COALESCE(ep.title, mo.title) AS title,
            sh.id AS show_id, sh.title AS show_title,
            ep.number AS episode_number, se.number AS season_number FROM watch_history wh
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
            let (url, media) = match media_kind {
                MediaKind::Movie => {
                    let media = RecentlyWatchedEntryMedia::Movie { title: row.get(3) };
                    (format!("/movie/{}", row.get::<_, i32>(2)), media)
                }
                MediaKind::Episode => {
                    // TODO: page for specific episodes...
                    let show_id = row
                        .get::<_, Option<i32>>(4)
                        .expect("show_id null in watch_history row");

                    let media = RecentlyWatchedEntryMedia::Episode {
                        title: row.get(3),
                        number: row.get(6),
                        season_number: row.get(7),
                        show_title: row.get(5),
                    };

                    (format!("/show/{}", show_id), media)
                }
                _ => unreachable!("invalid media_kind in watch_history table"),
            };

            RecentlyWatchedEntry {
                watched_at: row.get(0),
                url,
                media,
            }
        })
        .collect();

    Ok(HtmlTemplate(IndexTemplate { recently_watched }))
}
