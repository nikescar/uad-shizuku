// Android ClipboardManager integration for clipboard operations
// Reference: https://developer.android.com/reference/android/content/ClipboardManager

#[cfg(target_os = "android")]
use jni::objects::{JObject, JValue};

#[cfg(target_os = "android")]
use ndk_context;

/// Get text from clipboard
/// Returns Some(String) if clipboard contains text, None otherwise
#[cfg(target_os = "android")]
pub fn get_text() -> std::io::Result<Option<String>> {
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

    // Get ClipboardManager via getSystemService
    let context_class = env
        .find_class("android/content/Context")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to find Context class: {}", e),
            )
        })?;

    let clipboard_service = env
        .get_static_field(
            &context_class,
            "CLIPBOARD_SERVICE",
            "Ljava/lang/String;",
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get CLIPBOARD_SERVICE constant: {}", e),
            )
        })?;

    let clipboard_service_obj = clipboard_service.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get CLIPBOARD_SERVICE object: {}", e),
        )
    })?;

    let clipboard_manager = env
        .call_method(
            &activity,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&clipboard_service_obj)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getSystemService: {}", e),
            )
        })?;

    let clipboard_manager_obj = clipboard_manager.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get ClipboardManager object: {}", e),
        )
    })?;

    if clipboard_manager_obj.is_null() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "ClipboardManager is null",
        ));
    }

    // Check if clipboard has primary clip
    let has_primary_clip = env
        .call_method(
            &clipboard_manager_obj,
            "hasPrimaryClip",
            "()Z",
            &[],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call hasPrimaryClip: {}", e),
            )
        })?
        .z()
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get hasPrimaryClip result: {}", e),
            )
        })?;

    if !has_primary_clip {
        return Ok(None);
    }

    // Get primary clip
    let primary_clip = env
        .call_method(
            &clipboard_manager_obj,
            "getPrimaryClip",
            "()Landroid/content/ClipData;",
            &[],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getPrimaryClip: {}", e),
            )
        })?;

    let primary_clip_obj = primary_clip.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get primary clip object: {}", e),
        )
    })?;

    if primary_clip_obj.is_null() {
        return Ok(None);
    }

    // Get item count
    let item_count = env
        .call_method(&primary_clip_obj, "getItemCount", "()I", &[])
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getItemCount: {}", e),
            )
        })?
        .i()
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get item count: {}", e),
            )
        })?;

    if item_count == 0 {
        return Ok(None);
    }

    // Get first item
    let clip_item = env
        .call_method(
            &primary_clip_obj,
            "getItemAt",
            "(I)Landroid/content/ClipData$Item;",
            &[JValue::Int(0)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getItemAt: {}", e),
            )
        })?;

    let clip_item_obj = clip_item.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get clip item object: {}", e),
        )
    })?;

    if clip_item_obj.is_null() {
        return Ok(None);
    }

    // Get text from item
    let text = env
        .call_method(
            &clip_item_obj,
            "getText",
            "()Ljava/lang/CharSequence;",
            &[],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getText: {}", e),
            )
        })?;

    let text_obj = text.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get text object: {}", e),
        )
    })?;

    if text_obj.is_null() {
        return Ok(None);
    }

    // Convert CharSequence to String
    let text_string = env
        .call_method(&text_obj, "toString", "()Ljava/lang/String;", &[])
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call toString: {}", e),
            )
        })?;

    let text_string_obj = text_string.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get text string object: {}", e),
        )
    })?;

    let result: String = env
        .get_string(&jni::objects::JString::from(text_string_obj))
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to convert to Rust string: {}", e),
            )
        })?
        .into();

    Ok(Some(result))
}

/// Check if clipboard has text
#[cfg(target_os = "android")]
pub fn has_text() -> std::io::Result<bool> {
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

    // Get ClipboardManager
    let context_class = env
        .find_class("android/content/Context")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to find Context class: {}", e),
            )
        })?;

    let clipboard_service = env
        .get_static_field(
            &context_class,
            "CLIPBOARD_SERVICE",
            "Ljava/lang/String;",
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get CLIPBOARD_SERVICE constant: {}", e),
            )
        })?;

    let clipboard_service_obj = clipboard_service.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get CLIPBOARD_SERVICE object: {}", e),
        )
    })?;

    let clipboard_manager = env
        .call_method(
            &activity,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&clipboard_service_obj)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getSystemService: {}", e),
            )
        })?;

    let clipboard_manager_obj = clipboard_manager.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get ClipboardManager object: {}", e),
        )
    })?;

    if clipboard_manager_obj.is_null() {
        return Ok(false);
    }

    // Check if has text
    let has_primary_clip = env
        .call_method(
            &clipboard_manager_obj,
            "hasPrimaryClip",
            "()Z",
            &[],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call hasPrimaryClip: {}", e),
            )
        })?
        .z()
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get hasPrimaryClip result: {}", e),
            )
        })?;

    Ok(has_primary_clip)
}

/// Set text to clipboard
#[cfg(target_os = "android")]
pub fn set_text(text: &str) -> std::io::Result<()> {
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

    // Get ClipboardManager
    let context_class = env
        .find_class("android/content/Context")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to find Context class: {}", e),
            )
        })?;

    let clipboard_service = env
        .get_static_field(
            &context_class,
            "CLIPBOARD_SERVICE",
            "Ljava/lang/String;",
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get CLIPBOARD_SERVICE constant: {}", e),
            )
        })?;

    let clipboard_service_obj = clipboard_service.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get CLIPBOARD_SERVICE object: {}", e),
        )
    })?;

    let clipboard_manager = env
        .call_method(
            &activity,
            "getSystemService",
            "(Ljava/lang/String;)Ljava/lang/Object;",
            &[JValue::Object(&clipboard_service_obj)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call getSystemService: {}", e),
            )
        })?;

    let clipboard_manager_obj = clipboard_manager.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get ClipboardManager object: {}", e),
        )
    })?;

    if clipboard_manager_obj.is_null() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "ClipboardManager is null",
        ));
    }

    // Create ClipData with text
    let clip_data_class = env
        .find_class("android/content/ClipData")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to find ClipData class: {}", e),
            )
        })?;

    let label = env.new_string("text").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create label string: {}", e),
        )
    })?;

    let text_java = env.new_string(text).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create text string: {}", e),
        )
    })?;

    // Call ClipData.newPlainText(label, text)
    let clip_data = env
        .call_static_method(
            &clip_data_class,
            "newPlainText",
            "(Ljava/lang/CharSequence;Ljava/lang/CharSequence;)Landroid/content/ClipData;",
            &[JValue::Object(&label), JValue::Object(&text_java)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to call newPlainText: {}", e),
            )
        })?;

    let clip_data_obj = clip_data.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get ClipData object: {}", e),
        )
    })?;

    // Set primary clip
    env.call_method(
        &clipboard_manager_obj,
        "setPrimaryClip",
        "(Landroid/content/ClipData;)V",
        &[JValue::Object(&clip_data_obj)],
    )
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to call setPrimaryClip: {}", e),
        )
    })?;

    tracing::debug!("Text copied to clipboard");
    Ok(())
}

// Non-Android stub implementations
#[cfg(not(target_os = "android"))]
pub fn get_text() -> std::io::Result<Option<String>> {
    Ok(None)
}

#[cfg(not(target_os = "android"))]
pub fn has_text() -> std::io::Result<bool> {
    Ok(false)
}

#[cfg(not(target_os = "android"))]
pub fn set_text(_text: &str) -> std::io::Result<()> {
    Ok(())
}
