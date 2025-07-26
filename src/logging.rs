use std::io::Write;

use anyhow::{Result, anyhow};
use log::LevelFilter;

/// Initialize the global logger based on configuration
///
/// # Errors
/// Returns an error if the logger cannot be initialized.
pub fn init_logging(level_filter: LevelFilter) -> Result<()> {
    // Simple stdout-only logging using env_logger
    env_logger::Builder::new()
        .filter_level(level_filter)
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] [{}:{}] {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .try_init()
        .map_err(|e| anyhow!("Failed to initialize stdout logger: {e}"))?;

    log::info!("Logging initialized: level={level_filter:?}");

    Ok(())
}
