use std::sync::{Arc, OnceLock, RwLock};

/// Target prefixes for noisy transitive dependencies (pulled in by eframe/egui's Linux
/// accessibility bridge and dark-mode detection) that spam INFO/DEBUG logs of their own
/// (DBus handshakes, connection polling, etc.) unrelated to app behavior. Capped at WARN
/// regardless of the app's configured level, unless the user explicitly asks for TRACE.
///
/// "tracing::span" / "tracing::event" are `tracing`'s own log-facade fallback targets: since
/// this app never installs a `tracing_subscriber`, any dependency using `tracing` (zbus, its
/// DBus stack, etc.) falls back to emitting through the `log` crate under these fixed target
/// names instead of the crate's own name, which is why they aren't caught by a crate-name match.
const NOISY_TARGET_PREFIXES: &[&str] = &[
    "zbus",
    "atspi",
    "accesskit_unix",
    "ashpd",
    "async_io",
    "async_executor",
    "polling",
    "tracing::span",
    "tracing::event",
];

/// Combined logger that sends logs to both logcat (on Android) and in-app UI capture
struct CombinedLogger {
    level_filter: Arc<RwLock<log::LevelFilter>>,
}

impl log::Log for CombinedLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let Ok(filter) = self.level_filter.read() else {
            return false;
        };

        if *filter != log::LevelFilter::Trace
            && NOISY_TARGET_PREFIXES
                .iter()
                .any(|prefix| metadata.target().starts_with(prefix))
        {
            return metadata.level() <= log::LevelFilter::Warn;
        }

        metadata.level() <= *filter
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // Format the log message
        let level = record.level();
        let target = record.target();
        let message = format!("{}", record.args());

        // Skip empty messages
        if message.trim().is_empty() {
            return;
        }

        // Format the complete log line
        let log_line = format!("[{}] {}: {}", level, target, message);

        // Convert level to string
        let level_str = match level {
            log::Level::Error => "ERROR",
            log::Level::Warn => "WARN",
            log::Level::Info => "INFO",
            log::Level::Debug => "DEBUG",
            log::Level::Trace => "TRACE",
        };

        // Append to global log buffer with level filtering
        crate::uad_shizuku_app::append_log(level_str, log_line.clone());

        // On Android, also send to logcat via android_logger
        #[cfg(target_os = "android")]
        {
            // Use println/eprintln as backup - android_logger redirects these to logcat
            match level {
                log::Level::Error => eprintln!("[{}] {}", target, message),
                _ => println!("[{}] {}", target, message),
            }
        }

        // On non-Android, just print to stdout/stderr
        #[cfg(not(target_os = "android"))]
        {
            match level {
                log::Level::Error => eprintln!("{}", log_line),
                _ => println!("{}", log_line),
            }
        }
    }

    fn flush(&self) {}
}

static LOGGER: OnceLock<CombinedLogger> = OnceLock::new();

/// Initialize the combined logger that writes to both logcat and in-app log capture
pub fn init_combined_logger(level_filter: log::LevelFilter) {
    let logger = LOGGER.get_or_init(|| CombinedLogger {
        level_filter: Arc::new(RwLock::new(level_filter)),
    });

    // Set as the global logger
    if log::set_logger(logger).is_ok() {
        log::set_max_level(level_filter);
    }

    // On Android, also initialize android_logger as a backup
    #[cfg(target_os = "android")]
    {
        let _ = android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(level_filter)
                .with_tag("UAD-Shizuku"),
        );
    }
}

/// Update the log level at runtime
pub fn update_log_level(level: &str) {
    eprintln!("DEBUG: update_log_level called with: {}", level);

    let level_filter = match level.to_uppercase().as_str() {
        "TRACE" => log::LevelFilter::Trace,
        "DEBUG" => log::LevelFilter::Debug,
        "INFO" => log::LevelFilter::Info,
        "WARN" => log::LevelFilter::Warn,
        "ERROR" => log::LevelFilter::Error,
        _ => log::LevelFilter::Error,
    };

    eprintln!("DEBUG: Converted to filter: {:?}", level_filter);

    if let Some(logger) = LOGGER.get() {
        eprintln!("DEBUG: Logger found, updating...");
        if let Ok(mut filter) = logger.level_filter.write() {
            *filter = level_filter;
            log::set_max_level(level_filter);
            eprintln!("DEBUG: Log level updated to {:?}", level_filter);
        } else {
            eprintln!("DEBUG: Failed to acquire write lock on filter");
        }
    } else {
        eprintln!("DEBUG: Logger not initialized yet!");
    }
}
