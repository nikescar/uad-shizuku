use crate::models::{GooglePlayApp, NewGooglePlayApp};
use anyhow::{Context, Result};
use diesel::prelude::*;
use std::time::{SystemTime, UNIX_EPOCH};

/// Get Google Play app from database by package ID
pub fn get_google_play_app(
    conn: &mut SqliteConnection,
    pkg_id: &str,
) -> Result<Option<GooglePlayApp>> {
    use crate::schema::google_play_apps::dsl::*;

    let result = google_play_apps
        .filter(package_id.eq(pkg_id))
        .first::<GooglePlayApp>(conn)
        .optional()
        .context("Failed to query Google Play app")?;

    Ok(result)
}

/// Insert or update Google Play app in database
pub fn upsert_google_play_app(
    conn: &mut SqliteConnection,
    pkg_id: &str,
    title_val: &str,
    developer_val: &str,
    version_val: Option<&str>,
    icon_base64_val: Option<&str>,
    score_val: Option<f32>,
    installs_val: Option<&str>,
    updated_val: Option<i32>,
    raw_response_val: &str,
) -> Result<GooglePlayApp> {
    use crate::schema::google_play_apps::dsl::*;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32;

    // Check if app exists
    let existing = get_google_play_app(conn, pkg_id)?;

    if let Some(_existing_app) = existing {
        // Update existing record
        diesel::update(google_play_apps.filter(package_id.eq(pkg_id)))
            .set((
                title.eq(title_val),
                developer.eq(developer_val),
                version.eq(version_val),
                icon_base64.eq(icon_base64_val),
                score.eq(score_val),
                installs.eq(installs_val),
                updated.eq(updated_val),
                raw_response.eq(raw_response_val),
                updated_at.eq(now),
            ))
            .execute(conn)
            .context("Failed to update Google Play app")?;

        log::info!("Updated Google Play app: {}", pkg_id);
    } else {
        // Insert new record
        let new_app = NewGooglePlayApp {
            package_id: pkg_id,
            title: title_val,
            developer: developer_val,
            version: version_val,
            icon_base64: icon_base64_val,
            score: score_val,
            installs: installs_val,
            updated: updated_val,
            raw_response: raw_response_val,
            created_at: now,
            updated_at: now,
        };

        diesel::insert_into(google_play_apps)
            .values(&new_app)
            .execute(conn)
            .context("Failed to insert Google Play app")?;

        log::info!("Inserted Google Play app: {}", pkg_id);
    }

    // Fetch and return the updated/inserted record
    get_google_play_app(conn, pkg_id)?.context("Failed to fetch Google Play app after upsert")
}

/// Delete Google Play app from database
pub fn delete_google_play_app(conn: &mut SqliteConnection, pkg_id: &str) -> Result<usize> {
    use crate::schema::google_play_apps::dsl::*;

    let count = diesel::delete(google_play_apps.filter(package_id.eq(pkg_id)))
        .execute(conn)
        .context("Failed to delete Google Play app")?;

    Ok(count)
}

/// Get all Google Play apps from database
pub fn get_all_google_play_apps(conn: &mut SqliteConnection) -> Result<Vec<GooglePlayApp>> {
    use crate::schema::google_play_apps::dsl::*;

    let results = google_play_apps
        .load::<GooglePlayApp>(conn)
        .context("Failed to query all Google Play apps")?;

    Ok(results)
}

/// Check if cache is stale (older than 7 days)
pub fn is_cache_stale(app: &GooglePlayApp) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32;

    let age_seconds = now - app.updated_at;
    let seven_days = 7 * 24 * 60 * 60;

    age_seconds > seven_days
}
