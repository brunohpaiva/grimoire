use std::sync::Arc;

use askama::Template;
use askama_web::WebTemplate;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use crate::AppState;

#[derive(Deserialize)]
pub struct SearchParams {
    query: String,
}

struct SearchResultEntry {
    title: String,
}

#[derive(Template, WebTemplate)]
#[template(path = "search_result.html")]
pub struct SearchResultTemplate {
    title: String,
    results: Vec<SearchResultEntry>,
}

pub async fn get_search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<SearchResultTemplate, Response> {
    let search_response = state
        .tmdb_api
        .multi_search(&params.query)
        .await
        .inspect_err(|err| println!("{}", err))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())?;

    let results = search_response
        .results
        .iter()
        .map(|entry| SearchResultEntry {
            title: entry.title.to_string(),
        })
        .collect();

    Ok(SearchResultTemplate {
        title: params.query,
        results,
    })
}
