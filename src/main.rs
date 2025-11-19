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
        let path = std::path::Path::new(&args[1]);
        if !path.exists() {
            eprintln!("Config file does not exist: {}", args[1]);
            return Ok(());
        }
        if !path.is_file() {
            eprintln!("Config file must be a regular file: {}", args[1]);
            return Ok(());
        }
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("toml"))
        {
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
        eprintln!("Warning: Could not load config.toml ({e}), using defaults");
        Config::default()
    });
    let config = config.with_env()?;

    // Initialize logging based on config
    random_image_server::init_logging(config.server.log_level)?;

    // Create and start the server
    let server = ImageServer::with_config(config);

    // Create a termination handler to gracefully shut down the server
    let (_terminator, mut interrupt_rx) = create_termination();

    if let Err(e) = server.start(interrupt_rx.resubscribe()).await {
        tracing::error!("Server encountered an unexpected error: {e}");
        return Err(e);
    }

    // Wait for termination signal
    if let Ok(reason) = interrupt_rx.recv().await {
        match reason {
            Interrupted::UserInt => tracing::info!("exited per user request"),
            Interrupted::OsSigInt => tracing::info!("exited because of an os sig int"),
            Interrupted::OsSigTerm => tracing::info!("exited because of an os sig term"),
            Interrupted::OsSigQuit => tracing::info!("exited because of an os sig quit"),
        }
    } else {
        tracing::error!("exited because of an unexpected error");
    }

    Ok(())
}
