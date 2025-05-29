use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use config::AppConfig;
use deadpool_postgres::Pool;

pub mod config;
mod db;
mod routes;
pub mod tmdb;
pub mod trakt;

struct AppState {
    pub pool: Pool,
    pub tmdb_api: tmdb::TmdbApi,
}

pub async fn start_server(config: AppConfig) -> Result<()> {
    let pool = db::create_pool(&config)?;
    let tmdb_api = tmdb::TmdbApi::new(&config.tmdb_api_key);

    let state = Arc::new(AppState { pool, tmdb_api });

    let app = Router::new()
        .merge(routes::main::build_router())
        .with_state(state);

    println!("Running server on {}", config.addr);

    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
