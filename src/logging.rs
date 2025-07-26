use std::io::Write;

use anyhow::{Result, anyhow};
use log::LevelFilter;

use crate::config::LogLevel;

/// Convert our `LogLevel` enum to `log::LevelFilter`
impl From<LogLevel> for LevelFilter {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => Self::Trace,
            LogLevel::Debug => Self::Debug,
            LogLevel::Info => Self::Info,
            LogLevel::Warn => Self::Warn,
            LogLevel::Error => Self::Error,
            LogLevel::Off => Self::Off,
        }
    }
}

/// Initialize the global logger based on configuration
///
/// # Errors
/// Returns an error if the logger cannot be initialized.
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
        .map_err(|e| anyhow!("Failed to initialize stdout logger: {e}"))?;

    log::info!("Logging initialized: level={log_level:?}");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_to_level_filter_conversion() {
        assert_eq!(LevelFilter::from(LogLevel::Trace), LevelFilter::Trace);
        assert_eq!(LevelFilter::from(LogLevel::Debug), LevelFilter::Debug);
        assert_eq!(LevelFilter::from(LogLevel::Info), LevelFilter::Info);
        assert_eq!(LevelFilter::from(LogLevel::Warn), LevelFilter::Warn);
        assert_eq!(LevelFilter::from(LogLevel::Error), LevelFilter::Error);
        assert_eq!(LevelFilter::from(LogLevel::Off), LevelFilter::Off);
    }
}
