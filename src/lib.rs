use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use config::AppConfig;
use deadpool_postgres::Pool;

pub mod config;
mod db;
mod routes;
pub mod trakt;

struct AppState {
    pub pool: Pool,
}

pub async fn start_server(config: AppConfig) -> Result<()> {
    let pool = db::create_pool(&config)?;

    let state = Arc::new(AppState { pool });

    let app = Router::new()
        .merge(routes::main::build_router())
        .with_state(state);

    println!("Running server on {}", config.addr);

    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
