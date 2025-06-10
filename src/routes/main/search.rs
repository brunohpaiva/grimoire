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
    tmdb::{SearchResultEntry, SearchResultMedia},
};

#[derive(Deserialize)]
pub struct SearchParams {
    query: String,
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
        .inspect_err(|err| eprintln!("{:?}", err))
        .map_err(|err| AppError::Internal(err.into()))?;

    Ok(HtmlTemplate(SearchResultTemplate {
        title: params.query,
        results: search_response.results,
    }))
}
