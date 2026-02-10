use std::sync::{Arc, OnceLock, RwLock};

/// Combined logger that sends logs to both logcat (on Android) and in-app UI capture
struct CombinedLogger {
    level_filter: Arc<RwLock<log::LevelFilter>>,
}

impl log::Log for CombinedLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        if let Ok(filter) = self.level_filter.read() {
            metadata.level() <= *filter
        } else {
            false
        }
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
    let level_filter = match level.to_uppercase().as_str() {
        "TRACE" => log::LevelFilter::Trace,
        "DEBUG" => log::LevelFilter::Debug,
        "INFO" => log::LevelFilter::Info,
        "WARN" => log::LevelFilter::Warn,
        "ERROR" => log::LevelFilter::Error,
        _ => log::LevelFilter::Error,
    };

    if let Some(logger) = LOGGER.get() {
        if let Ok(mut filter) = logger.level_filter.write() {
            *filter = level_filter;
            log::set_max_level(level_filter);
        }
    }
}
