use crate::models::{ApkMirrorApp, NewApkMirrorApp};
use anyhow::{Context, Result};
use diesel::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get APKMirror app from database by package ID
pub fn get_apkmirror_app(
    conn: &mut SqliteConnection,
    pkg_id: &str,
) -> Result<Option<ApkMirrorApp>> {
    use crate::schema::apkmirror_apps::dsl::*;

    let result = apkmirror_apps
        .filter(package_id.eq(pkg_id))
        .first::<ApkMirrorApp>(conn)
        .optional()
        .context("Failed to query APKMirror app")?;

    Ok(result)
}

/// Insert or update APKMirror app in database
pub fn upsert_apkmirror_app(
    conn: &mut SqliteConnection,
    pkg_id: &str,
    title_val: &str,
    developer_val: &str,
    version_val: Option<&str>,
    icon_url_val: Option<&str>,
    icon_base64_val: Option<&str>,
    raw_response_val: &str,
) -> Result<ApkMirrorApp> {
    use crate::schema::apkmirror_apps::dsl::*;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32;

    // Check if app exists
    let existing = get_apkmirror_app(conn, pkg_id)?;

    if let Some(_existing_app) = existing {
        // Update existing record
        diesel::update(apkmirror_apps.filter(package_id.eq(pkg_id)))
            .set((
                title.eq(title_val),
                developer.eq(developer_val),
                version.eq(version_val),
                icon_url.eq(icon_url_val),
                icon_base64.eq(icon_base64_val),
                raw_response.eq(raw_response_val),
                updated_at.eq(now),
            ))
            .execute(conn)
            .context("Failed to update APKMirror app")?;

        log::info!("Updated APKMirror app: {}", pkg_id);
    } else {
        // Insert new record
        let new_app = NewApkMirrorApp {
            package_id: pkg_id,
            title: title_val,
            developer: developer_val,
            version: version_val,
            icon_url: icon_url_val,
            icon_base64: icon_base64_val,
            raw_response: raw_response_val,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(apkmirror_apps)
            .values(&new_app)
            .execute(conn)
            .context("Failed to insert APKMirror app")?;

        log::info!("Inserted APKMirror app: {}", pkg_id);
    }

    // Fetch and return the updated/inserted record
    get_apkmirror_app(conn, pkg_id)?.context("Failed to fetch APKMirror app after upsert")
}

/// Delete APKMirror app from database
pub fn delete_apkmirror_app(conn: &mut SqliteConnection, pkg_id: &str) -> Result<usize> {
    use crate::schema::apkmirror_apps::dsl::*;

    let count = diesel::delete(apkmirror_apps.filter(package_id.eq(pkg_id)))
        .execute(conn)
        .context("Failed to delete APKMirror app")?;

    Ok(count)
}

/// Get all APKMirror apps from database
pub fn get_all_apkmirror_apps(conn: &mut SqliteConnection) -> Result<Vec<ApkMirrorApp>> {
    use crate::schema::apkmirror_apps::dsl::*;

    let results = apkmirror_apps
        .load::<ApkMirrorApp>(conn)
        .context("Failed to query all APKMirror apps")?;

    Ok(results)
}

/// Check if cache is stale (older than 7 days)
pub fn is_cache_stale(app: &ApkMirrorApp) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32;

    let age_seconds = now - app.updated_at;
    let seven_days = 7 * 24 * 60 * 60;

    age_seconds > seven_days
}
