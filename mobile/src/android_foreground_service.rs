// Rust JNI bridge to ForegroundKeepAliveService.java.
// Starts/stops a foreground service that keeps the process alive while the
// NativeActivity is backgrounded (e.g. device sleep), preventing the OS from
// reclaiming the process and losing in-memory app state on resume.
// On other platforms, these are no-ops.

#[cfg(target_os = "android")]
use jni::objects::GlobalRef;

#[cfg(target_os = "android")]
use jni::objects::JValue;

#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
static FOREGROUND_SERVICE_CLASS: OnceLock<GlobalRef> = OnceLock::new();

#[cfg(target_os = "android")]
fn get_jni_env() -> Result<(jni::JavaVM, jni::AttachGuard<'static>), std::io::Error> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm() as _) }.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Failed to get JVM: {}", e),
        )
    })?;
    let env: jni::AttachGuard<'static> = unsafe {
        std::mem::transmute(vm.attach_current_thread().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to attach thread: {}", e),
            )
        })?)
    };
    Ok((vm, env))
}

/// Initialize the ForegroundKeepAliveService class reference.
/// This should be called early during app startup when JNI env has correct classloader.
#[cfg(target_os = "android")]
fn init_foreground_service_bridge() {
    if FOREGROUND_SERVICE_CLASS.get().is_some() {
        return; // Already initialized
    }

    let Ok((_vm, mut env)) = get_jni_env() else {
        log::error!("Failed to get JNI env for ForegroundKeepAliveService initialization");
        return;
    };

    let ctx = ndk_context::android_context();
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };

    let Ok(class_loader) = env
        .call_method(
            &activity,
            "getClassLoader",
            "()Ljava/lang/ClassLoader;",
            &[],
        )
        .and_then(|v| v.l())
    else {
        log::error!("Failed to get class loader");
        return;
    };

    let Ok(class_name) = env.new_string("pe.nikescar.uad_shizuku.ForegroundKeepAliveService")
    else {
        log::error!("Failed to create class name string");
        return;
    };

    let bridge_class = match env.call_method(
        &class_loader,
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(&class_name)],
    ) {
        Ok(class) => match class.l() {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to extract class object: {}", e);
                return;
            }
        },
        Err(e) => {
            log::error!("Failed to load ForegroundKeepAliveService class: {}", e);
            return;
        }
    };

    match env.new_global_ref(bridge_class) {
        Ok(global_ref) => {
            let _ = FOREGROUND_SERVICE_CLASS.set(global_ref);
            log::info!("ForegroundKeepAliveService class initialized successfully");
        }
        Err(e) => {
            log::error!("Failed to create global ref: {}", e);
        }
    }
}

#[cfg(target_os = "android")]
fn get_bridge_class() -> Result<&'static GlobalRef, std::io::Error> {
    if let Some(class_ref) = FOREGROUND_SERVICE_CLASS.get() {
        return Ok(class_ref);
    }

    init_foreground_service_bridge();

    FOREGROUND_SERVICE_CLASS.get().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "ForegroundKeepAliveService class initialization failed",
        )
    })
}

/// Start the foreground keep-alive service so the process survives device sleep.
#[cfg(target_os = "android")]
pub fn start_foreground_keep_alive_service() -> std::io::Result<()> {
    let (_vm, mut env) = get_jni_env()?;
    let class = get_bridge_class()?;
    let jclass: &jni::objects::JClass = class.as_obj().into();

    let ctx = ndk_context::android_context();
    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };

    // Best-effort: request POST_NOTIFICATIONS (API 33+) so the foreground
    // service's notification is actually visible to the user. A denial (or
    // failure on older API levels where the method is a no-op) does not
    // block the service itself from keeping the process alive.
    if let Err(e) = env.call_static_method(
        jclass,
        "requestNotificationPermission",
        "(Landroid/app/Activity;)V",
        &[JValue::Object(&activity)],
    ) {
        log::warn!("Failed to request notification permission: {}", e);
    }

    env.call_static_method(
        jclass,
        "startService",
        "(Landroid/content/Context;)V",
        &[JValue::Object(&activity)],
    )
    .map(|_| ())
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("startService call failed: {}", e),
        )
    })
}

/// Stop the foreground keep-alive service (best-effort, e.g. on clean app exit).
#[cfg(target_os = "android")]
pub fn stop_foreground_keep_alive_service() -> std::io::Result<()> {
    let (_vm, mut env) = get_jni_env()?;
    let class = get_bridge_class()?;
    let jclass: &jni::objects::JClass = class.as_obj().into();

    let ctx = ndk_context::android_context();
    let context = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };

    env.call_static_method(
        jclass,
        "stopService",
        "(Landroid/content/Context;)V",
        &[JValue::Object(&context)],
    )
    .map(|_| ())
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("stopService call failed: {}", e),
        )
    })
}

#[cfg(not(target_os = "android"))]
pub fn start_foreground_keep_alive_service() -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn stop_foreground_keep_alive_service() -> std::io::Result<()> {
    Ok(())
}
