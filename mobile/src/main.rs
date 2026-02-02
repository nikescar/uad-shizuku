use eframe::egui;
use uad_shizuku::uad_shizuku_app::{self, UadShizukuApp};

/// Check OpenGL version on Windows and show installation instructions if OpenGL 2.0+ is not available
#[cfg(target_os = "windows")]
fn check_opengl_and_show_instructions() {
    use std::process::Command;

    // Check OpenGL version using PowerShell
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            r#"
            Add-Type -TypeDefinition @'
            using System;
            using System.Runtime.InteropServices;
            public class OpenGL {
                [DllImport("opengl32.dll", SetLastError = true)]
                public static extern IntPtr wglCreateContext(IntPtr hdc);
                [DllImport("opengl32.dll", SetLastError = true)]
                public static extern bool wglMakeCurrent(IntPtr hdc, IntPtr hglrc);
                [DllImport("opengl32.dll", SetLastError = true)]
                public static extern bool wglDeleteContext(IntPtr hglrc);
                [DllImport("opengl32.dll", SetLastError = true)]
                public static extern IntPtr glGetString(uint name);
                [DllImport("user32.dll", SetLastError = true)]
                public static extern IntPtr GetDC(IntPtr hWnd);
                [DllImport("gdi32.dll", SetLastError = true)]
                public static extern int ChoosePixelFormat(IntPtr hdc, ref PIXELFORMATDESCRIPTOR ppfd);
                [DllImport("gdi32.dll", SetLastError = true)]
                public static extern bool SetPixelFormat(IntPtr hdc, int format, ref PIXELFORMATDESCRIPTOR ppfd);
                [StructLayout(LayoutKind.Sequential)]
                public struct PIXELFORMATDESCRIPTOR {
                    public ushort nSize, nVersion;
                    public uint dwFlags;
                    public byte iPixelType, cColorBits, cRedBits, cRedShift, cGreenBits, cGreenShift, cBlueBits, cBlueShift;
                    public byte cAlphaBits, cAlphaShift, cAccumBits, cAccumRedBits, cAccumGreenBits, cAccumBlueBits, cAccumAlphaBits;
                    public byte cDepthBits, cStencilBits, cAuxBuffers, iLayerType, bReserved;
                    public uint dwLayerMask, dwVisibleMask, dwDamageMask;
                }
            }
'@
            $hdc = [OpenGL]::GetDC([IntPtr]::Zero)
            $pfd = New-Object OpenGL+PIXELFORMATDESCRIPTOR
            $pfd.nSize = [System.Runtime.InteropServices.Marshal]::SizeOf($pfd)
            $pfd.nVersion = 1
            $pfd.dwFlags = 0x25  # PFD_DRAW_TO_WINDOW | PFD_SUPPORT_OPENGL | PFD_DOUBLEBUFFER
            $pfd.iPixelType = 0  # PFD_TYPE_RGBA
            $pfd.cColorBits = 32
            $pfd.cDepthBits = 24
            $format = [OpenGL]::ChoosePixelFormat($hdc, [ref]$pfd)
            if ($format -eq 0) { Write-Output "0.0"; exit }
            [OpenGL]::SetPixelFormat($hdc, $format, [ref]$pfd) | Out-Null
            $hglrc = [OpenGL]::wglCreateContext($hdc)
            if ($hglrc -eq [IntPtr]::Zero) { Write-Output "0.0"; exit }
            [OpenGL]::wglMakeCurrent($hdc, $hglrc) | Out-Null
            $versionPtr = [OpenGL]::glGetString(0x1F02)  # GL_VERSION
            if ($versionPtr -ne [IntPtr]::Zero) {
                $version = [System.Runtime.InteropServices.Marshal]::PtrToStringAnsi($versionPtr)
                Write-Output $version
            } else {
                Write-Output "0.0"
            }
            [OpenGL]::wglMakeCurrent([IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
            [OpenGL]::wglDeleteContext($hglrc) | Out-Null
            "#,
        ])
        .output();

    let gl_version_str = match output {
        Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        Err(_) => "0.0".to_string(),
    };

    // Parse the major.minor version from the OpenGL version string (e.g., "4.6.0 NVIDIA 537.42" -> 4.6)
    let version: f32 = gl_version_str
        .split_whitespace()
        .next()
        .and_then(|v| {
            let parts: Vec<&str> = v.split('.').collect();
            if parts.len() >= 2 {
                format!("{}.{}", parts[0], parts[1]).parse().ok()
            } else {
                v.parse().ok()
            }
        })
        .unwrap_or(0.0);

    if version < 2.0 {
        eprintln!("================================================================================");
        eprintln!("ERROR: OpenGL 2.0 or higher is required but not found on this system.");
        eprintln!("Detected OpenGL version: {}", if version > 0.0 { gl_version_str.as_str() } else { "Not available" });
        eprintln!("================================================================================");
        eprintln!();
        eprintln!("To install Mesa3D OpenGL drivers, run the following PowerShell commands as Administrator:");
        eprintln!();
        eprintln!("```powershell");
        eprintln!("# Download Mesa3D");
        eprintln!("Invoke-WebRequest -Uri 'https://github.com/pal1000/mesa-dist-win/releases/latest/download/mesa3d-25.3.3-release-msvc.7z' -OutFile \"$env:TEMP\\mesa3d.7z\"");
        eprintln!();
        eprintln!("# Extract (requires 7-Zip or expand-archive alternative)");
        eprintln!("# If you have 7-Zip installed:");
        eprintln!("& 'C:\\Program Files\\7-Zip\\7z.exe' x \"$env:TEMP\\mesa3d.7z\" -o\"$env:TEMP\\mesa3d\" -y");
        eprintln!();
        eprintln!("# Run the systemwide deployment (choose option 1 for Core desktop OpenGL drivers)");
        eprintln!("cd \"$env:TEMP\\mesa3d\"");
        eprintln!(".\\systemwidedeploy.cmd");
        eprintln!("# When prompted, enter: 1");
        eprintln!("# This installs Core desktop OpenGL drivers system-wide");
        eprintln!("```");
        eprintln!();
        eprintln!("After installation, restart this application.");
        eprintln!("================================================================================");

        // Show a message box as well for GUI users
        let _ = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                r#"
                Add-Type -AssemblyName System.Windows.Forms
                [System.Windows.Forms.MessageBox]::Show(
                    "OpenGL 2.0 or higher is required but not found.`n`nPlease install Mesa3D OpenGL drivers:`n1. Download from: https://github.com/pal1000/mesa-dist-win/releases/latest`n2. Extract mesa3d-25.3.3-release-msvc.7z`n3. Run systemwidedeploy.cmd as Administrator`n4. Choose option 1 (Core desktop OpenGL drivers)`n`nCheck the console for detailed instructions.",
                    "OpenGL Not Found - UAD-Shizuku",
                    [System.Windows.Forms.MessageBoxButtons]::OK,
                    [System.Windows.Forms.MessageBoxIcon]::Error
                )
                "#,
            ])
            .output();

        std::process::exit(1);
    } else {
        tracing::info!("OpenGL version detected: {}", gl_version_str);
    }
}

#[cfg(not(target_os = "windows"))]
fn check_opengl_and_show_instructions() {
    // On non-Windows platforms, OpenGL is typically provided by the system
    // No special check needed
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

    // Check OpenGL availability on Windows before initializing eframe
    check_opengl_and_show_instructions();

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
            uad_shizuku_app::init_egui(&cc.egui_ctx);
            Ok(Box::<UadShizukuApp>::default())
        }),
    )
}
