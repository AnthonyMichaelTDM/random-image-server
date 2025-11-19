use anyhow::{Result, anyhow};
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

/// Initialize the global tracing subscriber based on configuration
///
/// # Errors
/// Returns an error if the subscriber cannot be initialized.
pub fn init_logging(level: Level) -> Result<()> {
    // Simple stdout-only logging using tracing-subscriber
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_span_events(FmtSpan::NONE)
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .try_init()
        .map_err(|e| anyhow!("Failed to initialize tracing subscriber: {e}"))?;

    tracing::info!("Logging initialized: level={level:?}");

    Ok(())
}
