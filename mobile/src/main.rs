use eframe::egui;
use uad_shizuku::uad_shizuku_app::{self, UadShizukuApp};

/// Check OpenGL version on Windows and show installation instructions if OpenGL 2.0+ is not available
#[cfg(target_os = "windows")]
fn show_opengl_instructions() {
    use winsafe::{self as w, gui, co, prelude::*};

    /// Instruction dialog window
    #[derive(Clone)]
    struct InstructionWindow {
        wnd: gui::WindowMain,
        txt: gui::Edit,
        btn: gui::Button,
    }

    impl InstructionWindow {
        fn new() -> Self {
            let wnd = gui::WindowMain::new(
                gui::WindowMainOpts {
                    title: "UAD-Shizuku - OpenGL Required",
                    size: (500, 280),
                    style: gui::WindowMainOpts::default().style | co::WS::MINIMIZEBOX,
                    ..Default::default()
                },
            );

            let instruction_text = "System does not have OpenGL 2.0+.\r\n\r\n\
                To run this application, please install the Mesa3D OpenGL drivers:\r\n\r\n\
                https://uad-shizuku.github.io/docs/installation#download\r\n\r\n\
                After installation, restart this application.";

            let txt = gui::Edit::new(
                &wnd,
                gui::EditOpts {
                    position: (20, 20),
                    width: 460,
                    height: 180,
                    text: instruction_text,
                    window_style: co::WS::CHILD | co::WS::VISIBLE | co::WS::TABSTOP | co::WS::VSCROLL,
                    window_ex_style: co::WS_EX::CLIENTEDGE,
                    control_style: co::ES::MULTILINE | co::ES::READONLY | co::ES::AUTOVSCROLL,
                    resize_behavior: (gui::Horz::Resize, gui::Vert::Resize),
                    ..Default::default()
                },
            );

            let btn = gui::Button::new(
                &wnd,
                gui::ButtonOpts {
                    text: "&Exit",
                    position: (200, 220),
                    width: 100,
                    height: 30,
                    resize_behavior: (gui::Horz::Repos, gui::Vert::Repos),
                    ..Default::default()
                },
            );

            let new_self = Self { wnd, txt, btn };
            new_self.events();
            new_self
        }

        fn run(&self) -> w::AnyResult<i32> {
            self.wnd.run_main(None)
        }

        fn events(&self) {
            let wnd = self.wnd.clone();
            self.btn.on().bn_clicked(move || {
                wnd.hwnd().DestroyWindow()?;
                Ok(())
            });
        }
    }

    if let Err(e) = (|| InstructionWindow::new().run())() {
        w::HWND::NULL.MessageBox(
            &e.to_string(), "Error", co::MB::ICONERROR).unwrap();
    }

    std::process::exit(1);
}

#[cfg(target_os = "windows")]
fn hide_console() {
    use winapi::um::wincon::GetConsoleWindow;
    use winapi::um::winuser::{ShowWindow, SW_HIDE};
    unsafe {
        let console_window = GetConsoleWindow();
        if !console_window.is_null() {
            ShowWindow(console_window, SW_HIDE);
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_console() {
    // No-op on non-Windows platforms
}

fn main() -> eframe::Result<()> {
    // Hide console on Windows for GUI mode
    #[cfg(target_os = "windows")]
    hide_console();

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

    // Set panic hook on Windows to show OpenGL instructions if eframe panics during initialization
    #[cfg(target_os = "windows")]
    {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            default_hook(panic_info);
            show_opengl_instructions();
        }));
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    let result = eframe::run_native(
        "UAD-Shizuku",
        options,
        Box::new(|cc| {
            uad_shizuku_app::init_egui(&cc.egui_ctx);
            Ok(Box::<UadShizukuApp>::default())
        }),
    );

    // show OpenGL installation instructions if eframe failed to load on Windows
    #[cfg(target_os = "windows")]
    if result.is_err() {
        show_opengl_instructions();
    }

    result
}
