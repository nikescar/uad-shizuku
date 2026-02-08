// Android ScaleGestureDetector integration for pinch-to-zoom
// Reference: https://developer.android.com/reference/android/view/ScaleGestureDetector

#[cfg(target_os = "android")]
use jni::objects::{GlobalRef, JValue};

#[cfg(target_os = "android")]
use ndk_context;

#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
static HELPER_CLASS: OnceLock<GlobalRef> = OnceLock::new();

/// Load the ScaleGestureHelper class using the Activity's classloader.
/// NativeActivity's JNI thread uses the system classloader which can't find app classes,
/// so we must use activity.getClassLoader().loadClass() instead of env.find_class().
#[cfg(target_os = "android")]
fn init_helper_class() {
    if HELPER_CLASS.get().is_some() {
        return;
    }

    let ctx = ndk_context::android_context();
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm() as _) }) else {
        tracing::error!("ScaleGesture: failed to get JavaVM");
        return;
    };

    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };
    let Ok(mut env) = vm.attach_current_thread() else {
        tracing::error!("ScaleGesture: failed to attach thread");
        return;
    };

    let Ok(class_loader) = env
        .call_method(&activity, "getClassLoader", "()Ljava/lang/ClassLoader;", &[])
        .and_then(|v| v.l())
    else {
        tracing::error!("ScaleGesture: failed to get classloader");
        return;
    };

    let Ok(class_name) = env.new_string("pe.nikescar.uad_shizuku.ScaleGestureHelper") else {
        tracing::error!("ScaleGesture: failed to create class name string");
        return;
    };

    let helper_class = match env.call_method(
        &class_loader,
        "loadClass",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[JValue::Object(&class_name)],
    ) {
        Ok(class) => match class.l() {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("ScaleGesture: failed to extract class object: {}", e);
                return;
            }
        },
        Err(e) => {
            tracing::error!("ScaleGesture: failed to load ScaleGestureHelper class: {}", e);
            return;
        }
    };

    match env.new_global_ref(helper_class) {
        Ok(global_ref) => {
            let _ = HELPER_CLASS.set(global_ref);
            tracing::info!("ScaleGestureHelper class loaded successfully");
        }
        Err(e) => {
            tracing::error!("ScaleGesture: failed to create global ref: {}", e);
        }
    }
}

#[cfg(target_os = "android")]
fn get_helper_class() -> Result<&'static GlobalRef, std::io::Error> {
    if let Some(class_ref) = HELPER_CLASS.get() {
        return Ok(class_ref);
    }
    init_helper_class();
    HELPER_CLASS.get().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "ScaleGestureHelper class not loaded",
        )
    })
}

/// Initialize the ScaleGestureHelper by attaching a ScaleGestureDetector
/// to the Activity's content view. Call once during app startup.
#[cfg(target_os = "android")]
pub fn init() -> std::io::Result<()> {
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

    let class = get_helper_class()?;
    let jclass: &jni::objects::JClass = class.as_obj().into();

    env.call_static_method(
        jclass,
        "init",
        "(Landroid/app/Activity;)V",
        &[JValue::Object(&activity)],
    )
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to call ScaleGestureHelper.init: {}", e),
        )
    })?;

    tracing::info!("ScaleGestureHelper initialized");
    Ok(())
}

/// Get the accumulated scale factor since the last call and reset it.
/// Returns a multiplicative factor (~1.0 when idle, >1.0 spread, <1.0 pinch).
#[cfg(target_os = "android")]
pub fn get_scale_factor() -> f32 {
    let ctx = ndk_context::android_context();
    let Ok(vm) = (unsafe { jni::JavaVM::from_raw(ctx.vm() as _) }) else {
        return 1.0;
    };

    let Ok(mut env) = vm.attach_current_thread() else {
        return 1.0;
    };

    let Ok(class) = get_helper_class() else {
        return 1.0;
    };
    let jclass: &jni::objects::JClass = class.as_obj().into();

    let Ok(result) = env.call_static_method(jclass, "getAndResetScale", "()F", &[]) else {
        return 1.0;
    };

    result.f().unwrap_or(1.0)
}

// Non-Android stub implementations
#[cfg(not(target_os = "android"))]
pub fn init() -> std::io::Result<()> {
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn get_scale_factor() -> f32 {
    1.0
}
