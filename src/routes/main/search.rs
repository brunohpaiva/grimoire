use std::sync::Arc;

use askama::Template;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    AppState,
    response::{AppError, HtmlTemplate},
    tmdb::TmdbId,
};

#[derive(Deserialize)]
pub struct SearchParams {
    query: String,
}

struct SearchResultEntry {
    tmdb_id: TmdbId,
    title: String,
    // TODO: type this as MediaKind ?
    tmdb_type: String,
}

#[derive(Template)]
#[template(path = "search_result.html")]
pub struct SearchResultTemplate {
    title: String,
    results: Vec<SearchResultEntry>,
}

pub async fn get_search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<impl IntoResponse, AppError> {
    let search_response = state
        .tmdb_api
        .multi_search(&params.query)
        .await
        .map_err(|err| AppError::Internal(err.into()))?;

    let results = search_response
        .results
        .iter()
        .map(|entry| SearchResultEntry {
            tmdb_id: entry.id.clone(),
            title: entry.title.to_string(),
            tmdb_type: entry.media_type.to_string(),
        })
        .collect();

    Ok(HtmlTemplate(SearchResultTemplate {
        title: params.query,
        results,
    }))
}
