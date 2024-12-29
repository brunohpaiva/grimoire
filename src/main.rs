use std::process;

use grimoire::{start_server, AppConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::from_env().unwrap_or_else(|err| {
        eprintln!("Couldn't parse config: {err}");
        process::exit(1);
    });

    start_server(config).await
}
