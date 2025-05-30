use std::sync::Arc;

use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::AppState;

struct Episode {
    id: i32,
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
pub struct ShowTemplate {
    title: String,
    release_year: i32,
    seasons: Vec<Season>,
}

pub async fn get_show(
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
            SELECT sh.title AS show_title, sh.release_year AS show_release_year, 
            se.title AS season_title, se.number AS season_number,
            ep.id AS episode_id, ep.title AS episode_title, ep.number AS episode_number,
            COUNT(wh.watched_at) AS play_count FROM show sh
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

        let Some(season_number) = row.get::<_, Option<i32>>(3) else {
            continue;
        };
        let season_idx_opt = template
            .seasons
            .iter()
            .position(|season| season.number == season_number);

        let season = match season_idx_opt {
            Some(season_idx) => &mut template.seasons[season_idx],
            None => {
                let idx = template.seasons.len();
                template.seasons.push(Season {
                    title: row.get(2),
                    number: season_number,
                    episodes: vec![],
                });
                &mut template.seasons[idx]
            }
        };

        let Some(episode_id) = row.get::<_, Option<i32>>(4) else {
            continue;
        };

        season.episodes.push(Episode {
            id: episode_id,
            title: row.get(5),
            number: row.get(6),
            play_count: row.get(7),
        });
    }

    Ok(template)
}
