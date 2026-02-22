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
pub fn get_application_label(package_id: &str) -> std::io::Result<String> {
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

    // Get PackageManager
    let package_manager = env
        .call_method(
            &activity,
            "getPackageManager",
            "()Landroid/content/pm/PackageManager;",
            &[],
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get PackageManager: {}", e),
            )
        })?;

    // Create Java string for package name
    let j_package_id = env.new_string(package_id).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create Java string: {}", e),
        )
    })?;

    // Get ApplicationInfo: pm.getApplicationInfo(packageName, 0)
    let app_info = env
        .call_method(
            &package_manager,
            "getApplicationInfo",
            "(Ljava/lang/String;I)Landroid/content/pm/ApplicationInfo;",
            &[JValue::Object(&j_package_id), JValue::Int(0)],
        )
        .and_then(|v| v.l());
    // Clear pending JNI exception (getApplicationInfo throws NameNotFoundException for uninstalled packages)
    let _ = env.exception_clear();
    let app_info = app_info.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get ApplicationInfo for {}: {}", package_id, e),
        )
    })?;

    // Get label: pm.getApplicationLabel(applicationInfo)
    let label_cs = env
        .call_method(
            &package_manager,
            "getApplicationLabel",
            "(Landroid/content/pm/ApplicationInfo;)Ljava/lang/CharSequence;",
            &[JValue::Object(&app_info)],
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get application label for {}: {}", package_id, e),
            )
        })?;

    // Convert CharSequence to String via .toString()
    let label_jstring = env
        .call_method(&label_cs, "toString", "()Ljava/lang/String;", &[])
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to convert label to String for {}: {}", package_id, e),
            )
        })?;

    let label: String = env
        .get_string(&jni::objects::JString::from(label_jstring))
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to convert label to Rust string for {}: {}", package_id, e),
            )
        })?
        .into();

    Ok(label)
}

#[cfg(target_os = "android")]
pub fn get_application_icon(package_id: &str) -> std::io::Result<Vec<u8>> {
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

    // Get PackageManager
    let package_manager = env
        .call_method(
            &activity,
            "getPackageManager",
            "()Landroid/content/pm/PackageManager;",
            &[],
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get PackageManager: {}", e),
            )
        })?;

    // Create Java string for package name
    let j_package_id = env.new_string(package_id).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create Java string: {}", e),
        )
    })?;

    // Get ApplicationInfo
    let app_info = env
        .call_method(
            &package_manager,
            "getApplicationInfo",
            "(Ljava/lang/String;I)Landroid/content/pm/ApplicationInfo;",
            &[JValue::Object(&j_package_id), JValue::Int(0)],
        )
        .and_then(|v| v.l());
    // Clear pending JNI exception (getApplicationInfo throws NameNotFoundException for uninstalled packages)
    let _ = env.exception_clear();
    let app_info = app_info.map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to get ApplicationInfo for {}: {}", package_id, e),
        )
    })?;

    // Get icon drawable: pm.getApplicationIcon(applicationInfo)
    let drawable = env
        .call_method(
            &package_manager,
            "getApplicationIcon",
            "(Landroid/content/pm/ApplicationInfo;)Landroid/graphics/drawable/Drawable;",
            &[JValue::Object(&app_info)],
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get application icon for {}: {}", package_id, e),
            )
        })?;

    // Get intrinsic dimensions (may be -1 for AdaptiveIconDrawable)
    let width = env
        .call_method(&drawable, "getIntrinsicWidth", "()I", &[])
        .and_then(|v| v.i())
        .unwrap_or(96);
    let height = env
        .call_method(&drawable, "getIntrinsicHeight", "()I", &[])
        .and_then(|v| v.i())
        .unwrap_or(96);

    // Default to 96x96 if dimensions are invalid
    let width = if width <= 0 { 96 } else { width };
    let height = if height <= 0 { 96 } else { height };

    // Get Bitmap.Config.ARGB_8888
    let bitmap_config_class = env
        .find_class("android/graphics/Bitmap$Config")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to find Bitmap.Config class: {}", e),
            )
        })?;
    let argb_8888 = env
        .get_static_field(
            &bitmap_config_class,
            "ARGB_8888",
            "Landroid/graphics/Bitmap$Config;",
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get ARGB_8888 config: {}", e),
            )
        })?;

    // Create Bitmap: Bitmap.createBitmap(width, height, config)
    let bitmap_class = env.find_class("android/graphics/Bitmap").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to find Bitmap class: {}", e),
        )
    })?;
    let bitmap = env
        .call_static_method(
            &bitmap_class,
            "createBitmap",
            "(IILandroid/graphics/Bitmap$Config;)Landroid/graphics/Bitmap;",
            &[
                JValue::Int(width),
                JValue::Int(height),
                JValue::Object(&argb_8888),
            ],
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create Bitmap: {}", e),
            )
        })?;

    // Create Canvas: new Canvas(bitmap)
    let canvas_class = env.find_class("android/graphics/Canvas").map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to find Canvas class: {}", e),
        )
    })?;
    let canvas = env
        .new_object(
            &canvas_class,
            "(Landroid/graphics/Bitmap;)V",
            &[JValue::Object(&bitmap)],
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to create Canvas: {}", e),
            )
        })?;

    // Set drawable bounds: drawable.setBounds(0, 0, width, height)
    env.call_method(
        &drawable,
        "setBounds",
        "(IIII)V",
        &[
            JValue::Int(0),
            JValue::Int(0),
            JValue::Int(width),
            JValue::Int(height),
        ],
    )
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to set drawable bounds: {}", e),
        )
    })?;

    // Draw drawable on canvas: drawable.draw(canvas)
    env.call_method(
        &drawable,
        "draw",
        "(Landroid/graphics/Canvas;)V",
        &[JValue::Object(&canvas)],
    )
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to draw drawable: {}", e),
        )
    })?;

    // Create ByteArrayOutputStream
    let baos_class = env
        .find_class("java/io/ByteArrayOutputStream")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to find ByteArrayOutputStream class: {}", e),
            )
        })?;
    let baos = env.new_object(&baos_class, "()V", &[]).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create ByteArrayOutputStream: {}", e),
        )
    })?;

    // Get CompressFormat.PNG
    let compress_format_class = env
        .find_class("android/graphics/Bitmap$CompressFormat")
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to find CompressFormat class: {}", e),
            )
        })?;
    let png_format = env
        .get_static_field(
            &compress_format_class,
            "PNG",
            "Landroid/graphics/Bitmap$CompressFormat;",
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get PNG CompressFormat: {}", e),
            )
        })?;

    // Compress bitmap: bitmap.compress(PNG, 100, baos)
    env.call_method(
        &bitmap,
        "compress",
        "(Landroid/graphics/Bitmap$CompressFormat;ILjava/io/OutputStream;)Z",
        &[
            JValue::Object(&png_format),
            JValue::Int(100),
            JValue::Object(&baos),
        ],
    )
    .map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to compress bitmap: {}", e),
        )
    })?;

    // Get byte array: baos.toByteArray()
    let byte_array = env
        .call_method(&baos, "toByteArray", "()[B", &[])
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get byte array: {}", e),
            )
        })?;

    // Convert Java byte[] to Rust Vec<u8>
    let j_byte_array = unsafe { jni::objects::JByteArray::from_raw(byte_array.as_raw()) };
    let bytes = env.convert_byte_array(j_byte_array).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to convert byte array: {}", e),
        )
    })?;

    // Recycle bitmap to free native memory
    let _ = env.call_method(&bitmap, "recycle", "()V", &[]);

    Ok(bytes)
}

#[cfg(target_os = "android")]
pub fn get_installer_package_name(package_id: &str) -> std::io::Result<Option<String>> {
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

    // Get PackageManager
    let package_manager = env
        .call_method(
            &activity,
            "getPackageManager",
            "()Landroid/content/pm/PackageManager;",
            &[],
        )
        .and_then(|v| v.l())
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to get PackageManager: {}", e),
            )
        })?;

    // Create Java string for package name
    let j_package_id = env.new_string(package_id).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to create Java string: {}", e),
        )
    })?;

    // Call getInstallerPackageName
    let installer_result = env
        .call_method(
            &package_manager,
            "getInstallerPackageName",
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[JValue::Object(&j_package_id)],
        );

    // Clear pending JNI exception (may throw if package not found)
    let _ = env.exception_clear();

    match installer_result {
        Ok(result) => {
            match result.l() {
                Ok(installer_obj) => {
                    if installer_obj.is_null() {
                        // null means developer install (sideloaded)
                        Ok(None)
                    } else {
                        // Convert to Rust string
                        let installer_str: String = env
                            .get_string(&jni::objects::JString::from(installer_obj))
                            .map_err(|e| {
                                std::io::Error::new(
                                    std::io::ErrorKind::Other,
                                    format!("Failed to convert installer name to Rust string: {}", e),
                                )
                            })?
                            .into();
                        Ok(Some(installer_str))
                    }
                }
                Err(_) => Ok(None),
            }
        }
        Err(_) => Ok(None),
    }
}
