use crate::api_virustotal::{LastAnalysisStats, VirusTotalResponse};
use crate::models::{NewVirusTotalResult, VirusTotalResult};
use crate::schema::virustotal_results;
use diesel::prelude::*;
use std::error::Error;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

lazy_static::lazy_static! {
    static ref UPSERT_QUEUE: Arc<Mutex<Option<Sender<UpsertTask>>>> = Arc::new(Mutex::new(None));
}

struct UpsertTask {
    package_name: String,
    file_path: String,
    sha256: String,
    vt_response: VirusTotalResponse,
}

/// Initialize the upsert queue worker thread
pub fn init_upsert_queue() {
    let mut queue_lock = UPSERT_QUEUE.lock().unwrap();
    if queue_lock.is_some() {
        return; // Already initialized
    }

    let (tx, rx): (Sender<UpsertTask>, Receiver<UpsertTask>) = channel();
    *queue_lock = Some(tx);
    drop(queue_lock);

    // Spawn worker thread
    thread::spawn(move || {
        log::info!("VirusTotal upsert queue worker started");
        let mut conn = crate::db::establish_connection();

        for task in rx {
            match upsert_result_internal(
                &mut conn,
                &task.package_name,
                &task.file_path,
                &task.sha256,
                &task.vt_response,
            ) {
                Ok(_) => {
                    log::debug!(
                        "Successfully upserted VT result for {} ({})",
                        task.package_name,
                        task.file_path
                    );
                }
                Err(e) => {
                    log::error!(
                        "Failed to upsert VT result for {} ({}): {}",
                        task.package_name,
                        task.file_path,
                        e
                    );
                }
            }
        }

        log::info!("VirusTotal upsert queue worker stopped");
    });
}

/// Queue an upsert operation (non-blocking)
pub fn queue_upsert(
    package_name: String,
    file_path: String,
    sha256: String,
    vt_response: VirusTotalResponse,
) -> Result<(), Box<dyn Error>> {
    let queue_lock = UPSERT_QUEUE.lock().unwrap();
    if let Some(ref tx) = *queue_lock {
        let task = UpsertTask {
            package_name,
            file_path,
            sha256,
            vt_response,
        };
        tx.send(task).map_err(|e| Box::new(e) as Box<dyn Error>)?;
        Ok(())
    } else {
        Err("Upsert queue not initialized".into())
    }
}

/// Get all VirusTotal results for a package by package name
pub fn get_results_by_package(
    conn: &mut SqliteConnection,
    package_name: &str,
) -> Result<Vec<VirusTotalResult>, Box<dyn Error>> {
    use crate::schema::virustotal_results::dsl;

    let results = dsl::virustotal_results
        .filter(dsl::package_name.eq(package_name))
        .load::<VirusTotalResult>(conn)?;

    Ok(results)
}

/// Get VirusTotal result from database by SHA256
pub fn get_result_by_sha256(
    conn: &mut SqliteConnection,
    sha256: &str,
) -> Result<Option<VirusTotalResult>, Box<dyn Error>> {
    use crate::schema::virustotal_results::dsl;

    let result = dsl::virustotal_results
        .filter(dsl::sha256.eq(sha256))
        .first::<VirusTotalResult>(conn)
        .optional()?;

    Ok(result)
}

/// Get VirusTotal result by package name, file path, and SHA256
pub fn get_result_by_package_file_sha256(
    conn: &mut SqliteConnection,
    package_name: &str,
    file_path: &str,
    sha256: &str,
) -> Result<Option<VirusTotalResult>, Box<dyn Error>> {
    use crate::schema::virustotal_results::dsl;

    let result = dsl::virustotal_results
        .filter(dsl::package_name.eq(package_name))
        .filter(dsl::file_path.eq(file_path))
        .filter(dsl::sha256.eq(sha256))
        .first::<VirusTotalResult>(conn)
        .optional()?;

    Ok(result)
}

/// Internal upsert function (blocking, used by queue worker)
fn upsert_result_internal(
    conn: &mut SqliteConnection,
    package_name: &str,
    file_path: &str,
    sha256: &str,
    vt_response: &VirusTotalResponse,
) -> Result<VirusTotalResult, Box<dyn Error>> {
    use crate::schema::virustotal_results::dsl;

    // Get stats with defaults if not available (file not yet analyzed)
    let default_stats = LastAnalysisStats {
        malicious: 0,
        suspicious: 0,
        undetected: 0,
        harmless: 0,
        timeout: 0,
        confirmed_timeout: 0,
        failure: 0,
        type_unsupported: 0,
    };
    let stats = vt_response
        .data
        .attributes
        .last_analysis_stats
        .as_ref()
        .unwrap_or(&default_stats);
    let last_analysis_date = vt_response.data.attributes.last_analysis_date.unwrap_or(0);
    let dex_count = vt_response
        .data
        .attributes
        .androguard
        .as_ref()
        .and_then(|a| a.risk_indicator.as_ref())
        .and_then(|r| r.apk.as_ref())
        .and_then(|a| a.dex);

    let raw_response = serde_json::to_string(&vt_response)?;

    // Check if record exists
    let existing = get_result_by_package_file_sha256(conn, package_name, file_path, sha256)?;

    if let Some(existing_record) = existing {
        // Update existing record
        diesel::update(dsl::virustotal_results.find(existing_record.id))
            .set((
                dsl::last_analysis_date.eq(last_analysis_date as i32),
                dsl::malicious.eq(stats.malicious),
                dsl::suspicious.eq(stats.suspicious),
                dsl::undetected.eq(stats.undetected),
                dsl::harmless.eq(stats.harmless),
                dsl::timeout.eq(stats.timeout),
                dsl::failure.eq(stats.failure),
                dsl::type_unsupported.eq(stats.type_unsupported),
                dsl::dex_count.eq(dex_count),
                dsl::reputation.eq(vt_response.data.attributes.reputation),
                dsl::raw_response.eq(&raw_response),
                dsl::updated_at.eq(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i32),
            ))
            .execute(conn)?;

        Ok(dsl::virustotal_results
            .find(existing_record.id)
            .first(conn)?)
    } else {
        // Insert new record
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i32;

        let new_result = NewVirusTotalResult {
            package_name,
            file_path,
            sha256,
            last_analysis_date: last_analysis_date as i32,
            malicious: stats.malicious,
            suspicious: stats.suspicious,
            undetected: stats.undetected,
            harmless: stats.harmless,
            timeout: stats.timeout,
            failure: stats.failure,
            type_unsupported: stats.type_unsupported,
            dex_count,
            reputation: vt_response.data.attributes.reputation,
            raw_response: &raw_response,
            created_at: current_time,
            updated_at: current_time,
        };

        diesel::insert_into(virustotal_results::table)
            .values(&new_result)
            .execute(conn)?;

        // Get the last inserted record
        Ok(dsl::virustotal_results.order(dsl::id.desc()).first(conn)?)
    }
}

/// Synchronous upsert (for immediate operations)
pub fn upsert_result(
    conn: &mut SqliteConnection,
    package_name: &str,
    file_path: &str,
    sha256: &str,
    vt_response: &VirusTotalResponse,
) -> Result<VirusTotalResult, Box<dyn Error>> {
    upsert_result_internal(conn, package_name, file_path, sha256, vt_response)
}

/// Delete all VirusTotal results for a package by package name
pub fn delete_results_by_package(
    conn: &mut SqliteConnection,
    package_name: &str,
) -> Result<usize, Box<dyn Error>> {
    use crate::schema::virustotal_results::dsl;

    let deleted =
        diesel::delete(dsl::virustotal_results.filter(dsl::package_name.eq(package_name)))
            .execute(conn)?;

    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::establish_connection;

    #[test]
    fn test_virustotal_db_operations() {
        let conn = &mut establish_connection();

        let json = include_str!("../../reference/virustotal_filereport_response.json");
        let vt_response: VirusTotalResponse = serde_json::from_str(json).unwrap();

        // Insert
        let result = upsert_result(
            conn,
            "com.linkedin.android",
            "/data/app/com.linkedin.android-1/base.apk",
            "6f2ca352440a0027b9f8ed014d40a1557df2b4b3d3e3fc06e574cc02ead982aa",
            &vt_response,
        )
        .unwrap();

        assert_eq!(result.package_name, "com.linkedin.android");
        assert_eq!(
            result.file_path,
            "/data/app/com.linkedin.android-1/base.apk"
        );
        assert_eq!(result.malicious, 0);
        assert_eq!(result.dex_count, Some(8));

        // Get by package name
        let retrieved = get_results_by_package(conn, "com.linkedin.android").unwrap();
        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].package_name, "com.linkedin.android");

        // Get by SHA256
        let retrieved_by_sha = get_result_by_sha256(
            conn,
            "6f2ca352440a0027b9f8ed014d40a1557df2b4b3d3e3fc06e574cc02ead982aa",
        )
        .unwrap()
        .unwrap();
        assert_eq!(retrieved_by_sha.package_name, "com.linkedin.android");

        // Delete
        let deleted = delete_results_by_package(conn, "com.linkedin.android").unwrap();
        assert_eq!(deleted, 1);
    }
}
