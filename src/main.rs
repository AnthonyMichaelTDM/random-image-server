use random_image_server::{ImageServer, config::Config};

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 2 {
        eprintln!("Usage: {} [config_file]", args[0]);
        return Ok(());
    }
    let config_file = if args.len() == 2 {
        if args[1] == "--help" || args[1] == "-h" {
            eprintln!("Usage: {} [config_file]", args[0]);
            return Ok(());
        }
        if args[1].ends_with(".toml") {
            &args[1]
        } else {
            eprintln!("Config file must be a .toml file");
            return Ok(());
        }
    } else {
        "config.toml"
    };

    // Try to load config from file, fall back to default if not found
    let config = Config::from_file(config_file).unwrap_or_else(|e| {
        eprintln!(
            "Warning: Could not load config.toml ({}), using defaults",
            e
        );
        Config::default()
    });

    // TODO: Initialize logging based on config

    // Create and start the server
    let server = ImageServer::with_config(config);
    server.start().await?;

    Ok(())
}
