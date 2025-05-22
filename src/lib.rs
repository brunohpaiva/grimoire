use std::sync::Arc;

use anyhow::Result;
use askama::Template;
use askama_web::WebTemplate;
use axum::{Router, routing::get};
use config::AppConfig;
use deadpool_postgres::{Config, Pool, Runtime, tokio_postgres::NoTls};

pub mod config;

struct AppState {
    pub pool: Pool,
}

pub async fn start_server(config: AppConfig) -> Result<()> {
    let pool = create_db_pool(&config)?;

    let state = Arc::new(AppState { pool });

    let app = Router::new().route("/", get(get_index)).with_state(state);

    println!("Running server on {}", config.addr);

    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_index() -> IndexTemplate {
    IndexTemplate {}
}

#[derive(Template, WebTemplate)]
#[template(path = "index.html")]
struct IndexTemplate;

fn create_db_pool(config: &AppConfig) -> Result<Pool> {
    let mut cfg = Config::new();
    cfg.host = Some(config.db_host.clone());
    cfg.port = Some(config.db_port.clone());
    cfg.dbname = Some(config.db_name.clone());
    cfg.user = Some(config.db_user.clone());
    cfg.password = Some(config.db_password.clone());

    Ok(cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
}
