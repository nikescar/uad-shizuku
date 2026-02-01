use std::sync::{Arc, OnceLock, RwLock};
use tracing::Subscriber;
use tracing_subscriber::layer::Context;
use tracing_subscriber::Layer;

pub struct LogCaptureLayer;

impl<S> Layer<S> for LogCaptureLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        // Format the log message
        let metadata = event.metadata();
        let level = metadata.level();
        let target = metadata.target();

        // Create a simple visitor to extract the message
        struct MessageVisitor {
            message: String,
        }

        impl tracing::field::Visit for MessageVisitor {
            fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
                if field.name() == "message" {
                    self.message = format!("{:?}", value);
                }
            }

            fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
                if field.name() == "message" {
                    self.message = value.to_string();
                }
            }
        }

        let mut visitor = MessageVisitor {
            message: String::new(),
        };

        event.record(&mut visitor);

        // Format the complete log line
        let log_line = format!("[{}] {}: {}", level, target, visitor.message);

        // Convert level to string for filtering
        let level_str = match *level {
            tracing::Level::ERROR => "ERROR",
            tracing::Level::WARN => "WARN",
            tracing::Level::INFO => "INFO",
            tracing::Level::DEBUG => "DEBUG",
            tracing::Level::TRACE => "TRACE",
        };

        // Append to global log buffer with level filtering
        crate::uad_shizuku_app::append_log(level_str, log_line);
    }
}

// Type-erased reload handle using a closure
type ReloadFn = Box<dyn Fn(&str) + Send + Sync>;

// Global reload handle for dynamic log level changes
static RELOAD_HANDLE: OnceLock<Arc<RwLock<Option<ReloadFn>>>> = OnceLock::new();

fn get_reload_handle() -> &'static Arc<RwLock<Option<ReloadFn>>> {
    RELOAD_HANDLE.get_or_init(|| Arc::new(RwLock::new(None)))
}

/// Store the reload handle for later use (type-erased)
pub fn set_reload_fn<F>(reload_fn: F)
where
    F: Fn(&str) + Send + Sync + 'static,
{
    if let Ok(mut handle) = get_reload_handle().write() {
        *handle = Some(Box::new(reload_fn));
    }
}

/// Update the tracing log level at runtime
pub fn update_tracing_level(level: &str) {
    if let Ok(handle) = get_reload_handle().read() {
        if let Some(ref reload_fn) = *handle {
            reload_fn(level);
        }
    }
}
