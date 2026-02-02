use eframe::egui;
use uad_shizuku::uad_shizuku_app::{self, UadShizukuApp};

/// Check OpenGL version on Windows and show installation instructions if OpenGL 2.0+ is not available
#[cfg(target_os = "windows")]
fn show_opengl_instructions() {
    eprintln!("================================================================================");
    eprintln!("System does not have Opengl 2.0+. To run this application, please install the Mesa3D OpenGL drivers.");
    eprintln!("https://uad-shizuku.github.io/docs/installation/windows/#install-mesa3d-opengl-drivers");
    eprintln!();
    eprintln!("After installation, restart this application.");
    eprintln!("================================================================================");

    // Wait for user to press Enter before exiting so they can read the console message
    eprintln!();
    eprintln!("Press Enter to exit...");
    let _ = std::io::stdin().read_line(&mut String::new());

    std::process::exit(1);
}

fn main() -> eframe::Result<()> {
    // Initialize tracing subscriber for structured logging with log capture and reload support
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::reload;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::EnvFilter;

    // Try to load user's log level from settings, default to "error" if not found
    let log_level = if let Ok(config) = uad_shizuku::Config::new() {
        if let Ok(settings) = config.load_settings() {
            settings.log_level.to_lowercase()
        } else {
            "error".to_string()
        }
    } else {
        "error".to_string()
    };

    // Create a reloadable filter layer for dynamic log level changes
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&log_level));
    let (filter, reload_handle) = reload::Layer::new(env_filter);

    // Store the reload handle for later use (type-erased via closure)
    uad_shizuku::log_capture::set_reload_fn(move |level: &str| {
        let new_filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("error"));
        if let Err(e) = reload_handle.reload(new_filter) {
            eprintln!("Failed to reload log filter: {}", e);
        }
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(uad_shizuku::log_capture::LogCaptureLayer)
        .init();

    // Initialize common app components (database, i18n)
    uad_shizuku_app::init_common();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "UAD-Shizuku",
        options,
        Box::new(|cc| {

            // hide terminal window on Windows GUI apps
            #[cfg(target_os = "windows")]
            {
                use winapi::um::wincon::FreeConsole;
                unsafe {
                    FreeConsole();
                }
            }

            uad_shizuku_app::init_egui(&cc.egui_ctx);
            Ok(Box::<UadShizukuApp>::default())
        }),
    )

    // show OpenGL installation instructions if needed
    #[cfg(target_os = "windows")]
    {
        show_opengl_instructions();
    }
}
