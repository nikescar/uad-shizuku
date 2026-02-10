// Android InputMethodManager integration for showing/hiding soft keyboard
// Reference: https://developer.android.com/reference/android/view/inputmethod/InputMethodManager

#[cfg(target_os = "android")]
use jni::objects::{JObject, JValue};

#[cfg(target_os = "android")]
use ndk_context;

/// Show the soft input keyboard
/// Calls InputMethodManager.showSoftInput(View, int flags)
#[cfg(target_os = "android")]
pub fn show_soft_input() -> std::io::Result<()> {
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

    // Get InputMethodManager via getSystemService
    // First, get the INPUT_METHOD_SERVICE constant
    let context_class = env
        .find_class("android/content/Context")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to find Context class: {}", e),
            )
        })?;

    let input_method_service = env
        .get_static_field(
            &context_class,
            "INPUT_METHOD_SERVICE",
            "Ljava/lang/String;",
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get INPUT_METHOD_SERVICE constant: {}", e),
            )
        })?;

    let input_method_service_obj = input_method_service.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get INPUT_METHOD_SERVICE object: {}", e),
        )
    })?;

    // Call getSystemService to get InputMethodManager
    let imm = env
        .call_method(
            &activity,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&input_method_service_obj)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getSystemService: {}", e),
            )
        })?;

    let imm_obj = imm.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get InputMethodManager object: {}", e),
        )
    })?;

    if imm_obj.is_null() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "InputMethodManager is null",
        ));
    }

    // Get the activity's window
    let window = env
        .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get Window: {}", e),
            )
        })?;

    let window_obj = window.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get Window object: {}", e),
        )
    })?;

    // Get the DecorView (root view of the window)
    let decor_view = env
        .call_method(&window_obj, "getDecorView", "()Landroid/view/View;", &[])
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get DecorView: {}", e),
            )
        })?;

    let decor_view_obj = decor_view.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get DecorView object: {}", e),
        )
    })?;

    // SHOW_IMPLICIT = 0x0001 (from InputMethodManager constants)
    // SHOW_FORCED = 0x0002
    let flags = 0x0001; // SHOW_IMPLICIT

    // Call showSoftInput(View, int)
    let result = env
        .call_method(
            &imm_obj,
            "showSoftInput",
            "(Landroid/view/View;I)Z",
            &[JValue::Object(&decor_view_obj), JValue::Int(flags)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call showSoftInput: {}", e),
            )
        })?;

    let success = result.z().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get showSoftInput result: {}", e),
        )
    })?;

    if success {
        log::debug!("Soft keyboard shown successfully");
    } else {
        log::warn!("showSoftInput returned false");
    }

    Ok(())
}

/// Hide the soft input keyboard
/// Calls InputMethodManager.hideSoftInputFromWindow(IBinder, int flags)
#[cfg(target_os = "android")]
pub fn hide_soft_input() -> std::io::Result<()> {
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

    // Get InputMethodManager
    let context_class = env
        .find_class("android/content/Context")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to find Context class: {}", e),
            )
        })?;

    let input_method_service = env
        .get_static_field(
            &context_class,
            "INPUT_METHOD_SERVICE",
            "Ljava/lang/String;",
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get INPUT_METHOD_SERVICE constant: {}", e),
            )
        })?;

    let input_method_service_obj = input_method_service.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get INPUT_METHOD_SERVICE object: {}", e),
        )
    })?;

    let imm = env
        .call_method(
            &activity,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&input_method_service_obj)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getSystemService: {}", e),
            )
        })?;

    let imm_obj = imm.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get InputMethodManager object: {}", e),
        )
    })?;

    if imm_obj.is_null() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "InputMethodManager is null",
        ));
    }

    // Get the activity's window
    let window = env
        .call_method(&activity, "getWindow", "()Landroid/view/Window;", &[])
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get Window: {}", e),
            )
        })?;

    let window_obj = window.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get Window object: {}", e),
        )
    })?;

    // Get the DecorView
    let decor_view = env
        .call_method(&window_obj, "getDecorView", "()Landroid/view/View;", &[])
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get DecorView: {}", e),
            )
        })?;

    let decor_view_obj = decor_view.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get DecorView object: {}", e),
        )
    })?;

    // Get the window token from the view
    let window_token = env
        .call_method(
            &decor_view_obj,
            "getWindowToken",
            "()Landroid/os/IBinder;",
            &[],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get window token: {}", e),
            )
        })?;

    let window_token_obj = window_token.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get window token object: {}", e),
        )
    })?;

    // HIDE_NOT_ALWAYS = 0 (from InputMethodManager constants)
    let flags = 0;

    // Call hideSoftInputFromWindow(IBinder, int)
    let result = env
        .call_method(
            &imm_obj,
            "hideSoftInputFromWindow",
            "(Landroid/os/IBinder;I)Z",
            &[JValue::Object(&window_token_obj), JValue::Int(flags)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call hideSoftInputFromWindow: {}", e),
            )
        })?;

    let success = result.z().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get hideSoftInputFromWindow result: {}", e),
        )
    })?;

    if success {
        log::debug!("Soft keyboard hidden successfully");
    } else {
        log::debug!("hideSoftInputFromWindow returned false (keyboard may not be showing)");
    }

    Ok(())
}

/// Toggle the soft input keyboard visibility
#[cfg(target_os = "android")]
pub fn toggle_soft_input() -> std::io::Result<()> {
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

    let context_class = env
        .find_class("android/content/Context")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to find Context class: {}", e),
            )
        })?;

    let input_method_service = env
        .get_static_field(
            &context_class,
            "INPUT_METHOD_SERVICE",
            "Ljava/lang/String;",
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get INPUT_METHOD_SERVICE constant: {}", e),
            )
        })?;

    let input_method_service_obj = input_method_service.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get INPUT_METHOD_SERVICE object: {}", e),
        )
    })?;

    let imm = env
        .call_method(
            &activity,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&input_method_service_obj)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getSystemService: {}", e),
            )
        })?;

    let imm_obj = imm.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get InputMethodManager object: {}", e),
        )
    })?;

    if imm_obj.is_null() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "InputMethodManager is null",
        ));
    }

    // SHOW_FORCED = 0x0002
    // HIDE_IMPLICIT_ONLY = 0x0001
    let show_flags = 0x0002;
    let hide_flags = 0x0001;

    // Call toggleSoftInput(int showFlags, int hideFlags)
    env.call_method(
        &imm_obj,
        "toggleSoftInput",
        "(II)V",
        &[JValue::Int(show_flags), JValue::Int(hide_flags)],
    )
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to call toggleSoftInput: {}", e),
        )
    })?;

    log::debug!("Toggled soft keyboard");
    Ok(())
}

// Non-Android stub implementations
#[cfg(not(target_os = "android"))]
pub fn show_soft_input() -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn hide_soft_input() -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn toggle_soft_input() -> std::io::Result<()> {
    Ok(())
}
