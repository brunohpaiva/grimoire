use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::{
    AppState,
    db::{MediaKind, WatchHistory, get_media_by_id, insert_watch_history},
};

#[derive(Deserialize)]
pub struct AddWatchParams {
    media_kind: MediaKind,
    id: i32,
}

pub async fn post_add_watch(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AddWatchParams>,
) -> Result<Redirect, Response> {
    let conn = state
        .pool
        .get()
        .await
        .inspect_err(|err| eprintln!("{:?}", err))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    let Some(media) = get_media_by_id(&conn, params.id, Some(params.media_kind))
        .await
        .inspect_err(|err| eprintln!("{:?}", err))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?
    else {
        return Err(StatusCode::NOT_FOUND.into_response());
    };

    insert_watch_history(
        &conn,
        &WatchHistory {
            media,
            watched_at: jiff::Timestamp::now(),
        },
    )
    .await
    .inspect_err(|err| eprintln!("{:?}", err))
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    Ok(Redirect::to("/"))
}
