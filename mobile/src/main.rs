//! Desktop entry point for UAD-Shizuku
//!
//! Command-line options:
//! - `--silent` flag: Performs silent installation and exits (desktop only)
//! - `--uninstall` flag: Performs uninstallation and exits (desktop only)
//! - `--log [FILE]`: Enable file logging (defaults to uad-shizuku.log if FILE not specified)

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use eframe::egui::{self, IconData};
use uad_shizuku::uad_shizuku_app::{self, UadShizukuApp};

/// Check OpenGL version on Windows and show installation instructions if OpenGL 2.0+ is not available
#[cfg(target_os = "windows")]
fn show_opengl_instructions() {
    use winsafe::{self as w, co, gui, prelude::*};

    /// Instruction dialog window
    #[derive(Clone)]
    struct InstructionWindow {
        wnd: gui::WindowMain,
        txt: gui::Edit,
        btn: gui::Button,
    }

    impl InstructionWindow {
        fn new() -> Self {
            let wnd = gui::WindowMain::new(gui::WindowMainOpts {
                title: "UAD-Shizuku - OpenGL Required",
                size: (500, 280),
                style: gui::WindowMainOpts::default().style | co::WS::MINIMIZEBOX,
                ..Default::default()
            });

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
                    window_style: co::WS::CHILD
                        | co::WS::VISIBLE
                        | co::WS::TABSTOP
                        | co::WS::VSCROLL,
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
        w::HWND::NULL
            .MessageBox(&e.to_string(), "Error", co::MB::ICONERROR)
            .unwrap();
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
    // Parse command-line arguments FIRST to check for explicit flags and log configuration
    let args: Vec<String> = std::env::args().collect();

    // Check for --log argument
    let log_file = {
        let mut log_file_opt: Option<String> = None;
        let mut i = 0;
        while i < args.len() {
            if args[i] == "--log" {
                // Check if there's a value after --log
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    log_file_opt = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    // --log flag present but no value, use default
                    log_file_opt = Some("uad-shizuku.log".to_string());
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
        log_file_opt
    };

    let silent_install = args.iter().any(|arg| arg == "--silent");
    let uninstall = args.iter().any(|arg| arg == "--uninstall");

    // Initialize logger
    if let Some(log_path) = &log_file {
        // File-based logging
        let log_path = if log_path.is_empty() {
            "uad-shizuku.log"
        } else {
            log_path.as_str()
        };

        let target = Box::new(
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .unwrap_or_else(|e| {
                    eprintln!("Failed to open log file '{}': {}", log_path, e);
                    std::process::exit(1);
                }),
        );

        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .target(env_logger::Target::Pipe(target))
            .init();

        log::info!(
            "UAD-Shizuku v{} starting with file logging",
            env!("CARGO_PKG_VERSION")
        );
    } else {
        // Initialize combined logger for in-app log viewer (use settings, default to ERROR)
        let log_level = if let Ok(config) = uad_shizuku::Config::new() {
            if let Ok(settings) = config.load_settings() {
                settings.log_level.to_uppercase()
            } else {
                "ERROR".to_string()
            }
        } else {
            "ERROR".to_string()
        };

        let level_filter = match log_level.as_str() {
            "TRACE" => log::LevelFilter::Trace,
            "DEBUG" => log::LevelFilter::Debug,
            "INFO" => log::LevelFilter::Info,
            "WARN" => log::LevelFilter::Warn,
            "ERROR" => log::LevelFilter::Error,
            _ => log::LevelFilter::Error,
        };

        uad_shizuku::log_capture::init_combined_logger(level_filter);
        log::info!("UAD-Shizuku v{} starting", env!("CARGO_PKG_VERSION"));
    }

    // Handle --uninstall argument (for Windows Add/Remove Programs)
    #[cfg(not(target_os = "android"))]
    if uninstall {
        log::info!("Running in uninstall mode (--uninstall flag)");

        use uad_shizuku::install;
        use uad_shizuku::install_stt::InstallResult;

        match install::do_uninstall() {
            InstallResult::Success(msg) => {
                log::info!("Uninstallation succeeded: {}", msg);
                println!("{}", msg);
                std::process::exit(0);
            }
            InstallResult::Error(err) => {
                log::error!("Uninstallation failed: {}", err);
                eprintln!("Uninstallation failed: {}", err);
                std::process::exit(1);
            }
        }
    }

    #[cfg(target_os = "android")]
    if uninstall {
        eprintln!("Uninstall not supported on Android");
        std::process::exit(1);
    }

    // Handle --silent install mode (desktop only)
    #[cfg(not(target_os = "android"))]
    if silent_install {
        log::info!("Running in silent install mode (--silent flag)");

        use uad_shizuku::install;
        use uad_shizuku::install_stt::InstallResult;

        match install::do_install() {
            InstallResult::Success(msg) => {
                log::info!("Silent installation succeeded: {}", msg);
                println!("{}", msg);
                std::process::exit(0);
            }
            InstallResult::Error(err) => {
                log::error!("Silent installation failed: {}", err);
                eprintln!("Installation failed: {}", err);
                std::process::exit(1);
            }
        }
    }

    #[cfg(target_os = "android")]
    if silent_install {
        eprintln!("Silent install not supported on Android");
        std::process::exit(1);
    }

    // Hide console on Windows for GUI mode (unless --log is specified)
    #[cfg(target_os = "windows")]
    if log_file.is_none() {
        hide_console();
    }

    // Initialize common app components (database, i18n)
    uad_shizuku_app::init_common();

    log::info!("Running in GUI mode");

    // Set panic hook on Windows to show OpenGL instructions if eframe panics during initialization
    #[cfg(target_os = "windows")]
    {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            default_hook(panic_info);
            show_opengl_instructions();
        }));
    }

    let icon =
        image::load_from_memory(include_bytes!("../app/src/main/play_store_512.png")).unwrap();
    let icon = IconData {
        width: icon.width(),
        height: icon.height(),
        rgba: icon.into_rgba8().into_raw(),
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_min_inner_size([400.0, 300.0])
            .with_icon(icon),
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
