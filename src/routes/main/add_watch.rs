use std::sync::Arc;

use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;

use crate::{
    AppState,
    db::{MediaKind, WatchHistory, get_media_by_id, insert_watch_history},
    response::AppError,
};

#[derive(Deserialize)]
pub struct AddWatchParams {
    media_kind: MediaKind,
    id: i32,
}

pub async fn post_add_watch(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AddWatchParams>,
) -> Result<Redirect, AppError> {
    let conn = state
        .pool
        .get()
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let Some(media) = get_media_by_id(&conn, params.id, Some(params.media_kind))
        .await
        .map_err(|err| AppError::Internal(err.into()))?
    else {
        return Err(AppError::NotFound);
    };

    insert_watch_history(
        &conn,
        &WatchHistory {
            media,
            watched_at: jiff::Timestamp::now(),
        },
    )
    .await
    .map_err(|err| AppError::Internal(err.into()))?;

    Ok(Redirect::to("/"))
}
