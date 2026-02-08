// usage

// reference
// https://developer.android.com/reference/android/content/pm/PackageManager
// getInstalledPackages
// getPackageInfo
// getApplicationIcon

#[cfg(target_os = "android")]
use jni::objects::JValue;

#[cfg(target_os = "android")]
use ndk_context;

#[cfg(target_os = "android")]
pub fn get_installed_packages() -> std::io::Result<Vec<String>> {
    let ctx = ndk_context::android_context();
    let vm = unsafe { jni::JavaVM::from_raw(ctx.vm() as _) }.map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Expected to find JVM via ndk_context crate",
        )
    })?;

    let activity = unsafe { jni::objects::JObject::from_raw(ctx.context() as _) };
    let mut env = vm.attach_current_thread().map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::Other, "Failed to attach current thread")
    })?;

    // Get PackageManager from the activity
    let package_manager = env
        .call_method(
            &activity,
            "getPackageManager",
            "()Landroid/content/pm/PackageManager;",
            &[],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get PackageManager: {}", e),
            )
        })?;

    // Call getInstalledPackages
    let packages_list = env
        .call_method(
            package_manager.l().map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to get PackageManager object: {}", e),
                )
            })?,
            "getInstalledPackages",
            "(I)Ljava/util/List;",
            &[JValue::Int(0)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get installed packages: {}", e),
            )
        })?;

    // Convert Java List to Rust Vec<String>
    let java_list = packages_list.l().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get packages list object: {}", e),
        )
    })?;
    let size = env
        .call_method(&java_list, "size", "()I", &[])
        .and_then(|v| v.i())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get list size: {}", e),
            )
        })?;

    let mut package_names = Vec::new();
    for i in 0..size {
        let package_info = env
            .call_method(
                &java_list,
                "get",
                "(I)Ljava/lang/Object;",
                &[JValue::Int(i)],
            )
            .and_then(|v| v.l())
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to get package info at index {}: {}", i, e),
                )
            })?;
        let package_name = env
            .call_method(&package_info, "packageName", "()Ljava/lang/String;", &[])
            .and_then(|v| v.l())
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to get package name at index {}: {}", i, e),
                )
            })?;
        let package_name_rust: String = env
            .get_string(&jni::objects::JString::from(package_name))
            .map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!(
                        "Failed to convert package name to Rust string at index {}: {}",
                        i, e
                    ),
                )
            })?
            .into();
        package_names.push(package_name_rust);
    }
    Ok(package_names)
}

#[cfg(target_os = "android")]
pub fn get_application_icon(_package_name: &str) -> std::io::Result<Vec<u8>> {
    // Implementation would go here
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "Not implemented",
    ))
}
