use std::error::Error;

use askama_axum::Template;
use axum::{routing::get, Router};

pub struct AppConfig {
    addr: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, Box<dyn Error>> {
        let addr = std::env::var("ADDRESS").unwrap_or("0.0.0.0:3000".to_string());

        Ok(Self { addr })
    }
}

pub async fn start_server(config: AppConfig) -> Result<(), Box<dyn Error>> {
    let app = Router::new().route("/", get(get_index));

    println!("Running server on {}", config.addr);

    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn get_index() -> IndexTemplate {
    IndexTemplate {}
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate;
