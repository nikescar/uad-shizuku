// Android Configuration integration for detecting system theme mode (day/night)
// Reference: https://developer.android.com/reference/android/content/res/Configuration

#[cfg(target_os = "android")]
use jni::objects::JValue;

#[cfg(target_os = "android")]
use ndk_context;

// Configuration.UI_MODE_NIGHT_MASK = 0x30
const UI_MODE_NIGHT_MASK: i32 = 0x30;
// Configuration.UI_MODE_NIGHT_NO = 0x10
const UI_MODE_NIGHT_NO: i32 = 0x10;
// Configuration.UI_MODE_NIGHT_YES = 0x20
const UI_MODE_NIGHT_YES: i32 = 0x20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiThemeMode {
    Light,
    Dark,
    Unspecified,
}

/// Get the current system UI theme mode (day/night)
/// Returns UiThemeMode::Light, UiThemeMode::Dark, or UiThemeMode::Unspecified
#[cfg(target_os = "android")]
pub fn get_ui_theme_mode() -> std::io::Result<UiThemeMode> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm() as _) }.map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Expected to find JVM via ndk_context crate",
        )
    })?;

    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };
    let mut env = vm.attach_current_thread().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to attach current thread",
        )
    })?;

    // Call getResources() to get Resources object
    let resources = env
        .call_method(
            &activity,
            "getResources",
            "()Landroid/content/res/Resources;",
            &[],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getResources: {}", e),
            )
        })?;

    let resources_obj = resources.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get Resources object: {}", e),
        )
    })?;

    if resources_obj.is_null() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Resources is null",
        ));
    }

    // Call getConfiguration() to get Configuration object
    let configuration = env
        .call_method(
            &resources_obj,
            "getConfiguration",
            "()Landroid/content/res/Configuration;",
            &[],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getConfiguration: {}", e),
            )
        })?;

    let configuration_obj = configuration.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get Configuration object: {}", e),
        )
    })?;

    if configuration_obj.is_null() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Configuration is null",
        ));
    }

    // Get uiMode field from Configuration
    let ui_mode = env
        .get_field(&configuration_obj, "uiMode", "I")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get uiMode field: {}", e),
            )
        })?;

    let ui_mode_value = ui_mode.i().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get uiMode value: {}", e),
        )
    })?;

    // Apply UI_MODE_NIGHT_MASK to get night mode bits
    let current_night_mode = ui_mode_value & UI_MODE_NIGHT_MASK;

    log::debug!(
        "UI mode value: 0x{:x}, night mode: 0x{:x}",
        ui_mode_value,
        current_night_mode
    );

    // Check against constants
    let theme_mode = match current_night_mode {
        UI_MODE_NIGHT_NO => {
            log::debug!("System theme mode: Light (UI_MODE_NIGHT_NO)");
            UiThemeMode::Light
        }
        UI_MODE_NIGHT_YES => {
            log::debug!("System theme mode: Dark (UI_MODE_NIGHT_YES)");
            UiThemeMode::Dark
        }
        _ => {
            log::debug!("System theme mode: Unspecified (unknown value)");
            UiThemeMode::Unspecified
        }
    };

    Ok(theme_mode)
}

// Non-Android stub implementation
#[cfg(not(target_os = "android"))]
pub fn get_ui_theme_mode() -> std::io::Result<UiThemeMode> {
    Ok(UiThemeMode::Unspecified)
}
