// Rust JNI bridge to ShizukuBridge.java for executing shell commands via Shizuku.
// On Android, calls static methods on pe.nikescar.uad_shizuku.ShizukuBridge.
// On other platforms, provides stub implementations that return errors.

#[cfg(target_os = "android")]
use jni::objects::JValue;

#[cfg(target_os = "android")]
use jni::objects::GlobalRef;

#[cfg(target_os = "android")]
use ndk_context;

#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
static SHIZUKU_BRIDGE_CLASS: OnceLock<GlobalRef> = OnceLock::new();

/// Initialize the ShizukuBridge class reference.
/// This should be called early during app startup when JNI env has correct classloader.
#[cfg(target_os = "android")]
pub fn init_shizuku_bridge() {
    if SHIZUKU_BRIDGE_CLASS.get().is_some() {
        return; // Already initialized
    }

    let Ok((_vm, mut env)) = get_jni_env() else {
        tracing::error!("Failed to get JNI env for ShizukuBridge initialization");
        return;
    };

    // Get the NativeActivity object
    let ctx = ndk_context::android_context();
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };

    // Get the activity's class
    let Ok(_activity_class) = env.get_object_class(&activity) else {
        tracing::error!("Failed to get activity class");
        return;
    };

    // Get the class loader from the activity
    let Ok(class_loader) = env
        .call_method(&activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
        .and_then(|v| v.l())
    else {
        tracing::error!("Failed to get class loader");
        return;
    };

    // Get the class name as a Java string
    let Ok(class_name) = env.new_string("pe.nikescar.uad_shizuku.ShizukuBridge") else {
        tracing::error!("Failed to create class name string");
        return;
    };

    // Load the ShizukuBridge class using the app's class loader
    let bridge_class = match env.call_method(
        &class_loader,
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(&class_name)],
    ) {
        Ok(class) => match class.l() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to extract class object: {}", e);
                return;
            }
        },
        Err(e) => {
            tracing::error!("Failed to load ShizukuBridge class: {}", e);
            return;
        }
    };

    // Create a global reference to keep the class alive
    match env.new_global_ref(bridge_class) {
        Ok(global_ref) => {
            let _ = SHIZUKU_BRIDGE_CLASS.set(global_ref);
            tracing::info!("ShizukuBridge class initialized successfully");
        }
        Err(e) => {
            tracing::error!("Failed to create global ref: {}", e);
        }
    }
}

#[cfg(target_os = "android")]
fn get_jni_env() -> Result<(jni::JavaVM, jni::AttachGuard<'static>), std::io::Error> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm() as _) }.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Failed to get JVM: {}", e),
        )
    })?;
    // SAFETY: The JavaVM outlives the AttachGuard because ndk_context holds a
    // reference for the lifetime of the process. We transmute the lifetime to
    // 'static so we can return both the VM and guard together. The guard must
    // not outlive the calling scope in practice.
    let env: jni::AttachGuard<'static> =
        unsafe { std::mem::transmute(vm.attach_current_thread().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to attach thread: {}", e),
            )
        })?) };
    Ok((vm, env))
}

#[cfg(target_os = "android")]
const BRIDGE_CLASS: &str = "pe/nikescar/uad_shizuku/ShizukuBridge";

/// Get the cached ShizukuBridge class reference.
/// Initializes on first call if not already initialized.
#[cfg(target_os = "android")]
fn get_bridge_class() -> Result<&'static GlobalRef, std::io::Error> {
    // Try to get existing class
    if let Some(class_ref) = SHIZUKU_BRIDGE_CLASS.get() {
        return Ok(class_ref);
    }

    // Initialize if not done yet
    init_shizuku_bridge();

    // Return the now-initialized class
    SHIZUKU_BRIDGE_CLASS.get().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "ShizukuBridge class initialization failed",
        )
    })
}

/// Initialize ShizukuBridge: register permission listener.
/// Call once during app startup.
#[cfg(target_os = "android")]
pub fn shizuku_init() {
    let Ok((_vm, mut env)) = get_jni_env() else {
        tracing::error!("shizuku_init: failed to get JNI env");
        return;
    };
    let Ok(class) = get_bridge_class() else {
        tracing::error!("shizuku_init: ShizukuBridge class not initialized");
        return;
    };
    let jclass: &jni::objects::JClass = class.as_obj().into();
    if let Err(e) = env.call_static_method(jclass, "init", "()V", &[]) {
        tracing::error!("ShizukuBridge.init() failed: {}", e);
    }
}

/// Check if Shizuku service is running and reachable.
#[cfg(target_os = "android")]
pub fn shizuku_is_available() -> bool {
    let Ok((_vm, mut env)) = get_jni_env() else {
        tracing::error!("shizuku_is_available: failed to get JNI env");
        return false;
    };
    let Ok(class) = get_bridge_class() else {
        tracing::error!("shizuku_is_available: ShizukuBridge class not initialized");
        return false;
    };
    let jclass: &jni::objects::JClass = class.as_obj().into();
    match env.call_static_method(jclass, "isAvailable", "()Z", &[])
        .and_then(|v| v.z())
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("shizuku_is_available JNI call failed: {}", e);
            false
        }
    }
}

/// Check if the app already has Shizuku permission.
#[cfg(target_os = "android")]
pub fn shizuku_has_permission() -> bool {
    let Ok((_vm, mut env)) = get_jni_env() else {
        tracing::error!("shizuku_has_permission: failed to get JNI env");
        return false;
    };
    let Ok(class) = get_bridge_class() else {
        tracing::error!("shizuku_has_permission: ShizukuBridge class not initialized");
        return false;
    };
    let jclass: &jni::objects::JClass = class.as_obj().into();
    match env.call_static_method(jclass, "hasPermission", "()Z", &[])
        .and_then(|v| v.z())
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("shizuku_has_permission JNI call failed: {}", e);
            false
        }
    }
}

/// Request Shizuku permission from the user.
#[cfg(target_os = "android")]
pub fn shizuku_request_permission() {
    let Ok((_vm, mut env)) = get_jni_env() else {
        tracing::error!("shizuku_request_permission: failed to get JNI env");
        return;
    };
    let Ok(class) = get_bridge_class() else {
        tracing::error!("shizuku_request_permission: ShizukuBridge class not initialized");
        return;
    };
    let jclass: &jni::objects::JClass = class.as_obj().into();
    if let Err(e) = env.call_static_method(jclass, "requestPermission", "()V", &[]) {
        tracing::error!("shizuku_request_permission JNI call failed: {}", e);
    }
}

/// Get the permission state from ShizukuBridge.
/// Returns: 0=unknown, 1=requesting, 2=granted, 3=denied
#[cfg(target_os = "android")]
pub fn shizuku_get_permission_state() -> i32 {
    let Ok((_vm, mut env)) = get_jni_env() else {
        return 0;
    };
    let Ok(class) = get_bridge_class() else {
        return 0;
    };
    let jclass: &jni::objects::JClass = class.as_obj().into();
    env.call_static_method(jclass, "getPermissionState", "()I", &[])
        .and_then(|v| v.i())
        .unwrap_or(0)
}

/// Get the bind state from ShizukuBridge.
/// Returns: 0=not bound, 1=binding, 2=bound, 3=failed
#[cfg(target_os = "android")]
pub fn shizuku_get_bind_state() -> i32 {
    let Ok((_vm, mut env)) = get_jni_env() else {
        return 0;
    };
    let Ok(class) = get_bridge_class() else {
        return 0;
    };
    let jclass: &jni::objects::JClass = class.as_obj().into();
    env.call_static_method(jclass, "getBindState", "()I", &[])
        .and_then(|v| v.i())
        .unwrap_or(0)
}

/// Start binding to the Shizuku ShellService (non-blocking).
/// Returns true only if already bound. Poll shizuku_get_bind_state() for progress.
#[cfg(target_os = "android")]
pub fn shizuku_bind_service() -> bool {
    let Ok((_vm, mut env)) = get_jni_env() else {
        tracing::error!("shizuku_bind_service: failed to get JNI env");
        return false;
    };
    let Ok(class) = get_bridge_class() else {
        tracing::error!("shizuku_bind_service: ShizukuBridge class not initialized");
        return false;
    };
    let jclass: &jni::objects::JClass = class.as_obj().into();
    match env.call_static_method(jclass, "bindService", "()Z", &[])
        .and_then(|v| v.z())
    {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("shizuku_bind_service JNI call failed: {}", e);
            false
        }
    }
}

/// Execute a shell command via the Shizuku service.
/// Returns the combined stdout+stderr output as a String.
#[cfg(target_os = "android")]
pub fn shizuku_exec(command: &str) -> std::io::Result<String> {
    let (_vm, mut env) = get_jni_env()?;
    let class = get_bridge_class()?;
    let jclass: &jni::objects::JClass = class.as_obj().into();

    let j_command = env.new_string(command).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create Java string: {}", e),
        )
    })?;

    let result = env
        .call_static_method(
            jclass,
            "execCommand",
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[JValue::Object(&j_command)],
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("execCommand call failed: {}", e),
            )
        })?;

    let output: String = env
        .get_string(&jni::objects::JString::from(result))
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to convert result string: {}", e),
            )
        })?
        .into();

    if output.starts_with("ERROR: ") {
        Err(std::io::Error::new(std::io::ErrorKind::Other, output))
    } else {
        Ok(output)
    }
}

/// Execute a shell command via Shizuku, writing output to a file.
/// Bypasses Binder IPC size limit for large command outputs.
/// Returns the file contents as a String on success.
#[cfg(target_os = "android")]
pub fn shizuku_exec_to_file(command: &str, output_path: &str) -> std::io::Result<String> {
    let (_vm, mut env) = get_jni_env()?;
    let class = get_bridge_class()?;
    let jclass: &jni::objects::JClass = class.as_obj().into();

    let j_command = env.new_string(command).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create Java string: {}", e),
        )
    })?;

    let j_output_path = env.new_string(output_path).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create Java string: {}", e),
        )
    })?;

    let result = env
        .call_static_method(
            jclass,
            "execCommandToFile",
            "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
            &[JValue::Object(&j_command), JValue::Object(&j_output_path)],
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("execCommandToFile call failed: {}", e),
            )
        })?;

    // Check if result is null (success) or contains error message
    if result.is_null() {
        // Success - read the output file
        std::fs::read_to_string(output_path)
    } else {
        let error_msg: String = env
            .get_string(&jni::objects::JString::from(result))
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to convert error string: {}", e),
                )
            })?
            .into();
        Err(std::io::Error::new(std::io::ErrorKind::Other, error_msg))
    }
}

/// Check if the ShellService is currently bound.
#[cfg(target_os = "android")]
pub fn shizuku_is_service_bound() -> bool {
    let Ok((_vm, mut env)) = get_jni_env() else {
        return false;
    };
    let Ok(class) = get_bridge_class() else {
        return false;
    };
    let jclass: &jni::objects::JClass = class.as_obj().into();
    env.call_static_method(jclass, "isServiceBound", "()Z", &[])
        .and_then(|v| v.z())
        .unwrap_or(false)
}

/// Unbind from the Shizuku ShellService and release resources.
#[cfg(target_os = "android")]
pub fn shizuku_unbind_service() {
    let Ok((_vm, mut env)) = get_jni_env() else {
        return;
    };
    let Ok(class) = get_bridge_class() else {
        return;
    };
    let jclass: &jni::objects::JClass = class.as_obj().into();
    let _ = env.call_static_method(jclass, "unbindService", "()V", &[]);
}

// --- Non-Android stubs ---

#[cfg(not(target_os = "android"))]
pub fn init_shizuku_bridge() {
    // No-op on non-Android platforms
}

#[cfg(not(target_os = "android"))]
pub fn shizuku_init() {}

#[cfg(not(target_os = "android"))]
pub fn shizuku_is_available() -> bool {
    false
}

#[cfg(not(target_os = "android"))]
pub fn shizuku_has_permission() -> bool {
    false
}

#[cfg(not(target_os = "android"))]
pub fn shizuku_request_permission() {}

#[cfg(not(target_os = "android"))]
pub fn shizuku_get_permission_state() -> i32 {
    0
}

#[cfg(not(target_os = "android"))]
pub fn shizuku_get_bind_state() -> i32 {
    0
}

#[cfg(not(target_os = "android"))]
pub fn shizuku_bind_service() -> bool {
    false
}

#[cfg(not(target_os = "android"))]
pub fn shizuku_exec(_command: &str) -> std::io::Result<String> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Shizuku not available on this platform",
    ))
}

#[cfg(not(target_os = "android"))]
pub fn shizuku_exec_to_file(_command: &str, _output_path: &str) -> std::io::Result<String> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Shizuku not available on this platform",
    ))
}

#[cfg(not(target_os = "android"))]
pub fn shizuku_is_service_bound() -> bool {
    false
}

#[cfg(not(target_os = "android"))]
pub fn shizuku_unbind_service() {}
