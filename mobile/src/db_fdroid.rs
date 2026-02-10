use crate::models::{FDroidApp, NewFDroidApp};
use anyhow::{Context, Result};
use diesel::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get F-Droid app from database by package ID
pub fn get_fdroid_app(conn: &mut SqliteConnection, pkg_id: &str) -> Result<Option<FDroidApp>> {
    use crate::schema::fdroid_apps::dsl::*;

    let result = fdroid_apps
        .filter(package_id.eq(pkg_id))
        .first::<FDroidApp>(conn)
        .optional()
        .context("Failed to query F-Droid app")?;

    Ok(result)
}

/// Insert or update F-Droid app in database
pub fn upsert_fdroid_app(
    conn: &mut SqliteConnection,
    pkg_id: &str,
    title_val: &str,
    developer_val: &str,
    version_val: Option<&str>,
    icon_base64_val: Option<&str>,
    description_val: Option<&str>,
    license_val: Option<&str>,
    updated_val: Option<i32>,
    raw_response_val: &str,
) -> Result<FDroidApp> {
    use crate::schema::fdroid_apps::dsl::*;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32;

    // Check if app exists
    let existing = get_fdroid_app(conn, pkg_id)?;

    if let Some(_existing_app) = existing {
        // Update existing record
        diesel::update(fdroid_apps.filter(package_id.eq(pkg_id)))
            .set((
                title.eq(title_val),
                developer.eq(developer_val),
                version.eq(version_val),
                icon_base64.eq(icon_base64_val),
                description.eq(description_val),
                license.eq(license_val),
                updated.eq(updated_val),
                raw_response.eq(raw_response_val),
                updated_at.eq(now),
            ))
            .execute(conn)
            .context("Failed to update F-Droid app")?;

        log::info!("Updated F-Droid app: {}", pkg_id);
    } else {
        // Insert new record
        let new_app = NewFDroidApp {
            package_id: pkg_id,
            title: title_val,
            developer: developer_val,
            version: version_val,
            icon_base64: icon_base64_val,
            description: description_val,
            license: license_val,
            updated: updated_val,
            raw_response: raw_response_val,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(fdroid_apps)
            .values(&new_app)
            .execute(conn)
            .context("Failed to insert F-Droid app")?;

        log::info!("Inserted F-Droid app: {}", pkg_id);
    }

    // Fetch and return the updated/inserted record
    get_fdroid_app(conn, pkg_id)?.context("Failed to fetch F-Droid app after upsert")
}

/// Delete F-Droid app from database
pub fn delete_fdroid_app(conn: &mut SqliteConnection, pkg_id: &str) -> Result<usize> {
    use crate::schema::fdroid_apps::dsl::*;

    let count = diesel::delete(fdroid_apps.filter(package_id.eq(pkg_id)))
        .execute(conn)
        .context("Failed to delete F-Droid app")?;

    Ok(count)
}

/// Get all F-Droid apps from database
pub fn get_all_fdroid_apps(conn: &mut SqliteConnection) -> Result<Vec<FDroidApp>> {
    use crate::schema::fdroid_apps::dsl::*;

    let results = fdroid_apps
        .load::<FDroidApp>(conn)
        .context("Failed to query all F-Droid apps")?;

    Ok(results)
}

/// Check if cache is stale (older than 7 days)
pub fn is_cache_stale(app: &FDroidApp) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32;

    let age_seconds = now - app.updated_at;
    let seven_days = 7 * 24 * 60 * 60;

    age_seconds > seven_days
}
