use std::sync::Arc;

use axum::{
    Router,
    extract::{MatchedPath, Request},
};
use config::AppConfig;
use deadpool_postgres::Pool;
use thiserror::Error;
use tower_http::trace::TraceLayer;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod config;
mod db;
mod filters;
mod response;
mod routes;
pub mod tmdb;
pub mod trakt;

struct AppState {
    pub pool: Pool,
    pub tmdb_api: tmdb::TmdbApi,
}

#[derive(Error, Debug)]
pub enum StartServerError {
    #[error("failed to create database connection pool")]
    CreateDbPool(#[source] deadpool_postgres::CreatePoolError),
    #[error("failed to bind port")]
    Bind(#[source] std::io::Error),
    #[error("failed to listen on port")]
    Listen(#[source] std::io::Error),
}

pub async fn start_server(config: AppConfig) -> Result<(), StartServerError> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let pool = db::create_pool(&config).map_err(StartServerError::CreateDbPool)?;
    let tmdb_api = tmdb::TmdbApi::new(&config.tmdb_api_key);

    let state = Arc::new(AppState { pool, tmdb_api });

    let app = Router::new()
        .merge(routes::main::build_router())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request| {
                    let method = req.method();
                    let uri = req.uri();

                    let matched_path = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(|matched_path| matched_path.as_str());

                    tracing::debug_span!("request", %method, %uri, matched_path)
                })
                .on_failure(()),
        )
        .with_state(state);

    info!("Running server on http://{}", config.addr);

    let listener = tokio::net::TcpListener::bind(config.addr)
        .await
        .map_err(StartServerError::Listen)?;
    axum::serve(listener, app)
        .await
        .map_err(StartServerError::Bind)?;

    Ok(())
}
