use random_image_server::{
    ImageServer,
    config::Config,
    termination::{Interrupted, create_termination},
};

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

    // Initialize logging based on config
    random_image_server::init_logging(config.server.log_level)?;

    // Create and start the server
    let server = ImageServer::with_config(config);

    // Create a termination handler to gracefully shut down the server
    let (_terminator, mut interrupt_rx) = create_termination();

    if let Err(e) = server.start(interrupt_rx.resubscribe()).await {
        log::error!("Server encountered an unexpected error: {}", e);
        return Err(e);
    }

    // Wait for termination signal
    if let Ok(reason) = interrupt_rx.recv().await {
        match reason {
            Interrupted::UserInt => log::info!("exited per user request"),
            Interrupted::OsSigInt => log::info!("exited because of an os sig int"),
            Interrupted::OsSigTerm => log::info!("exited because of an os sig term"),
            Interrupted::OsSigQuit => log::info!("exited because of an os sig quit"),
        }
    } else {
        log::error!("exited because of an unexpected error");
    }

    Ok(())
}
