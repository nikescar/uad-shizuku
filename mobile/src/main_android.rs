use android_activity::AndroidApp;
use eframe::NativeOptions;

use crate::log_capture;
use crate::uad_shizuku_app::{self, UadShizukuApp};
use crate::Config;

// Android entry point
#[no_mangle]
pub fn android_main(app: AndroidApp) {
    // Initialize tracing subscriber for Android with log capture and reload support
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::reload;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    // Try to load user's log level from settings, default to ERROR if not found
    let log_level = if let Ok(config) = Config::new() {
        if let Ok(settings) = config.load_settings() {
            settings.log_level.to_lowercase()
        } else {
            "error".to_string()
        }
    } else {
        "error".to_string()
    };

    // Create a reloadable filter layer for dynamic log level changes
    let env_filter = EnvFilter::try_new(&log_level).unwrap_or_else(|_| EnvFilter::new("error"));
    let (filter, reload_handle) = reload::Layer::new(env_filter);

    // Store the reload handle for later use (type-erased via closure)
    log_capture::set_reload_fn(move |level: &str| {
        let new_filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("error"));
        if let Err(e) = reload_handle.reload(new_filter) {
            eprintln!("Failed to reload log filter: {}", e);
        }
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(log_capture::LogCaptureLayer)
        .init();

    // Initialize common app components (database, i18n)
    uad_shizuku_app::init_common();

    // Initialize Android logger with max level (actual filtering done by tracing)
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Trace)
            .with_tag("UAD-Shizuku"),
    );

    // Log initialization message to confirm logging is working
    log::info!("Android logger initialized successfully");
    log::info!("Starting mobile application with egui");

    // Also use println! as backup logging method
    println!("UAD-Shizuku: Application starting");
    eprintln!("UAD-Shizuku: Error stream test");

    // Set up panic handler to catch crashes
    std::panic::set_hook(Box::new(|panic_info| {
        log::error!("PANIC OCCURRED: {}", panic_info);
        eprintln!("UAD-Shizuku PANIC: {}", panic_info);
        if let Some(location) = panic_info.location() {
            log::error!("Panic location: {}:{}", location.file(), location.line());
            eprintln!(
                "UAD-Shizuku PANIC LOCATION: {}:{}",
                location.file(),
                location.line()
            );
        }
    }));

    std::env::set_var("RUST_BACKTRACE", "full");

    let options = NativeOptions {
        android_app: Some(app),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    match eframe::run_native(
        "UAD-Shizuku",
        options,
        Box::new(|cc| {
            uad_shizuku_app::init_egui(&cc.egui_ctx);
            Ok(Box::<UadShizukuApp>::default())
        }),
    ) {
        Ok(_) => {
            log::info!("UadShizukuApp exited successfully");
        }
        Err(e) => {
            log::error!("UadShizukuApp failed: {}", e);
            eprintln!("UadShizukuApp failed: {}", e);
        }
    }
}
