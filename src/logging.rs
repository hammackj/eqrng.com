use std::fs;
use std::path::Path;
use tracing::Level;
use tracing_subscriber::fmt::time::UtcTime;

// Tracing appender for file-based, non-blocking logging
use tracing_appender::non_blocking::WorkerGuard;
use tracing_appender::rolling;

pub fn init_logging(
    config: &crate::config::LoggingConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    // Ensure log directory exists
    if let Some(log_dir) = Path::new(&config.file_path).parent() {
        fs::create_dir_all(log_dir)?;
    }

    // Parse log level
    let log_level = config.level.parse::<Level>().unwrap_or(Level::INFO);

    // Build a rolling file appender (daily rotation).
    // The appender will create files like "<log_dir>/<file_name>.<YYYY-MM-DD>"
    let file_path = Path::new(&config.file_path);
    let log_dir = file_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("eq_rng.log");

    let file_appender = rolling::daily(log_dir, file_name);

    // Convert to non-blocking writer and keep the guard alive for program lifetime.
    let (non_blocking, guard): (tracing_appender::non_blocking::NonBlocking, WorkerGuard) =
        tracing_appender::non_blocking(file_appender);

    // Prevent the guard from being dropped (which would stop the logging worker).
    // We intentionally leak it so it lives for the whole process lifetime.
    // Alternative: store the guard in a global/static if you want explicit management.
    Box::leak(Box::new(guard));

    // Initialize tracing_subscriber with the file writer
    tracing_subscriber::fmt()
        .with_timer(UtcTime::rfc_3339())
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_max_level(log_level)
        .with_writer(non_blocking)
        .init();

    tracing::info!("Logging initialized with level: {}", log_level);
    tracing::info!("Log file: {}", config.file_path);

    Ok(())
}

// Helper function to log database operations
pub fn log_db_operation(
    operation: &str,
    table: &str,
    id: Option<i64>,
    result: &Result<(), sqlx::Error>,
) {
    match result {
        Ok(_) => {
            if let Some(id_val) = id {
                tracing::debug!(
                    "Database operation '{}' on {} with id {} completed successfully",
                    operation,
                    table,
                    id_val
                );
            } else {
                tracing::debug!(
                    "Database operation '{}' on {} completed successfully",
                    operation,
                    table
                );
            }
        }
        Err(e) => {
            if let Some(id_val) = id {
                tracing::error!(
                    "Database operation '{}' on {} with id {} failed: {}",
                    operation,
                    table,
                    id_val,
                    e
                );
            } else {
                tracing::error!(
                    "Database operation '{}' on {} failed: {}",
                    operation,
                    table,
                    e
                );
            }
        }
    }
}

// Helper function to log admin actions
pub fn log_admin_action(action: &str, resource: &str, id: Option<i64>, user_info: Option<&str>) {
    let user_info = user_info.unwrap_or("unknown");
    if let Some(id_val) = id {
        tracing::info!(
            "Admin action: {} on {} with id {} by {}",
            action,
            resource,
            id_val,
            user_info
        );
    } else {
        tracing::info!("Admin action: {} on {} by {}", action, resource, user_info);
    }
}

// Helper function to log security events
pub fn log_security_event(event: &str, details: &str, level: Level) {
    match level {
        Level::DEBUG => tracing::debug!("Security event: {} - {}", event, details),
        Level::INFO => tracing::info!("Security event: {} - {}", event, details),
        Level::WARN => tracing::warn!("Security event: {} - {}", event, details),
        Level::ERROR => tracing::error!("Security event: {} - {}", event, details),
        _ => tracing::info!("Security event: {} - {}", event, details),
    }
}
