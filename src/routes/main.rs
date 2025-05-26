use std::{sync::Arc, vec};

use askama::Template;
use askama_web::WebTemplate;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};

use crate::{AppState, db::MediaKind};

pub fn build_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_index))
        // TODO: accept slugs for movie and show
        .route("/movie/{movie_id}", get(get_movie))
        .route("/show/{show_id}", get(get_show))
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

#[derive(Template, WebTemplate)]
#[template(path = "movie.html")]
struct MovieTemplate {
    title: String,
    release_year: i32,
    play_count: i64,
}

async fn get_movie(
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
            SELECT mo.title, mo.release_year, COUNT(wh.watched_at) AS play_count FROM movie mo
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
        title: row.get(0),
        release_year: row.get(1),
        play_count: row.get(2),
    })
}

struct Episode {
    title: String,
    number: i32,
    play_count: i64,
}

struct Season {
    title: String,
    number: i32,
    episodes: Vec<Episode>,
}

#[derive(Template, WebTemplate)]
#[template(path = "show.html")]
struct ShowTemplate {
    title: String,
    release_year: i32,
    seasons: Vec<Season>,
}

async fn get_show(
    State(state): State<Arc<AppState>>,
    Path(show_id): Path<i32>,
) -> Result<ShowTemplate, Response> {
    let conn = state
        .pool
        .get()
        .await
        .inspect_err(|err| eprintln!("{:?}", err))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    let mut template = ShowTemplate {
        title: String::new(),
        release_year: 0,
        seasons: vec![],
    };

    let rows = conn
        .query(
            "
            select sh.title AS show_title, sh.release_year AS show_release_year, 
            se.title AS season_title, se.number AS season_numer,
            ep.title AS episode_title, ep.number AS episode_number,
            COUNT(wh.watched_at) AS play_count from show sh
            LEFT JOIN season se ON se.show_id = sh.id
            LEFT JOIN episode ep ON ep.show_id = sh.id AND ep.season_id = se.id
            LEFT JOIN watch_history wh ON wh.media_id = ep.id AND wh.media_kind = 'EPISODE'
            WHERE sh.id = $1
            GROUP BY sh.id, se.id, ep.id
            ORDER BY se.number, ep.number
            ",
            &[&show_id],
        )
        .await
        .inspect_err(|err| eprintln!("{:?}", err))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    for (row_idx, row) in rows.iter().enumerate() {
        if row_idx == 0 {
            template.title = row.get(0);
            template.release_year = row.get(1);
        }

        let season_number: i32 = row.get(3);
        let season_idx = (season_number - 1) as usize;

        let season = if season_idx >= template.seasons.len() {
            template.seasons.push(Season {
                title: row.get(2),
                number: season_number,
                episodes: vec![],
            });
            &mut template.seasons[season_idx]
        } else {
            &mut template.seasons[season_idx]
        };

        season.episodes.push(Episode {
            title: row.get(4),
            number: row.get(5),
            play_count: row.get(6),
        });
    }

    Ok(template)
}
