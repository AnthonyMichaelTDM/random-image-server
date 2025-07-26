use std::io::Write;

use anyhow::{Result, anyhow};
use log::LevelFilter;

use crate::config::LogLevel;

/// Convert our LogLevel enum to log::LevelFilter
impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => LevelFilter::Trace,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Off => LevelFilter::Off,
        }
    }
}

/// Initialize the global logger based on configuration
pub fn init_logging(log_level: LogLevel) -> Result<()> {
    let level_filter: LevelFilter = log_level.into();

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
        .map_err(|e| anyhow!("Failed to initialize stdout logger: {}", e))?;

    log::info!("Logging initialized: level={:?}", log_level);

    Ok(())
}
