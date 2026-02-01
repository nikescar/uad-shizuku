// https://hybrid-analysis.com/my-account

use crate::api_hybridanalysis::HybridAnalysisReportResponse;
use crate::models::{HybridAnalysisResult, NewHybridAnalysisResult};
use crate::schema::hybridanalysis_results;
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
    ha_response: HybridAnalysisReportResponse,
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
        tracing::info!("Hybrid Analysis upsert queue worker started");
        let mut conn = crate::db::establish_connection();

        for task in rx {
            match upsert_result_internal(
                &mut conn,
                &task.package_name,
                &task.file_path,
                &task.sha256,
                &task.ha_response,
            ) {
                Ok(_) => {
                    tracing::debug!(
                        "Successfully upserted Hybrid Analysis result for {} ({})",
                        task.package_name,
                        task.file_path
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to upsert Hybrid Analysis result for {} ({}): {}",
                        task.package_name,
                        task.file_path,
                        e
                    );
                }
            }
        }

        tracing::info!("Hybrid Analysis upsert queue worker stopped");
    });
}

/// Queue an upsert operation (non-blocking)
pub fn queue_upsert(
    package_name: String,
    file_path: String,
    sha256: String,
    ha_response: HybridAnalysisReportResponse,
) -> Result<(), Box<dyn Error>> {
    let queue_lock = UPSERT_QUEUE.lock().unwrap();
    if let Some(ref tx) = *queue_lock {
        let task = UpsertTask {
            package_name,
            file_path,
            sha256,
            ha_response,
        };
        tx.send(task).map_err(|e| Box::new(e) as Box<dyn Error>)?;
        Ok(())
    } else {
        Err("Upsert queue not initialized".into())
    }
}

/// Get all Hybrid Analysis results for a package by package name
pub fn get_results_by_package(
    conn: &mut SqliteConnection,
    package_name: &str,
) -> Result<Vec<HybridAnalysisResult>, Box<dyn Error>> {
    use crate::schema::hybridanalysis_results::dsl;

    let results = dsl::hybridanalysis_results
        .filter(dsl::package_name.eq(package_name))
        .load::<HybridAnalysisResult>(conn)?;

    Ok(results)
}

/// Get Hybrid Analysis result from database by SHA256
pub fn get_result_by_sha256(
    conn: &mut SqliteConnection,
    sha256: &str,
) -> Result<Option<HybridAnalysisResult>, Box<dyn Error>> {
    use crate::schema::hybridanalysis_results::dsl;

    let result = dsl::hybridanalysis_results
        .filter(dsl::sha256.eq(sha256))
        .first::<HybridAnalysisResult>(conn)
        .optional()?;

    Ok(result)
}

/// Get Hybrid Analysis result by package name, file path, and SHA256
pub fn get_result_by_package_file_sha256(
    conn: &mut SqliteConnection,
    package_name: &str,
    file_path: &str,
    sha256: &str,
) -> Result<Option<HybridAnalysisResult>, Box<dyn Error>> {
    use crate::schema::hybridanalysis_results::dsl;

    let result = dsl::hybridanalysis_results
        .filter(dsl::package_name.eq(package_name))
        .filter(dsl::file_path.eq(file_path))
        .filter(dsl::sha256.eq(sha256))
        .first::<HybridAnalysisResult>(conn)
        .optional()?;

    Ok(result)
}

/// Internal upsert function (blocking, used by queue worker)
fn upsert_result_internal(
    conn: &mut SqliteConnection,
    package_name: &str,
    file_path: &str,
    sha256: &str,
    ha_response: &HybridAnalysisReportResponse,
) -> Result<HybridAnalysisResult, Box<dyn Error>> {
    upsert_result_with_error(conn, package_name, file_path, sha256, ha_response, None)
}

/// Internal upsert function with optional error message
fn upsert_result_with_error(
    conn: &mut SqliteConnection,
    package_name: &str,
    file_path: &str,
    sha256: &str,
    ha_response: &HybridAnalysisReportResponse,
    error_message: Option<&str>,
) -> Result<HybridAnalysisResult, Box<dyn Error>> {
    use crate::schema::hybridanalysis_results::dsl;

    let classification_tags_json = serde_json::to_string(&ha_response.classification_tags)?;
    let tags_json = serde_json::to_string(&ha_response.tags)?;
    let raw_response = serde_json::to_string(&ha_response)?;

    // Use provided error_message or derive from ha_response.error_type
    let error_msg = error_message.map(|s| s.to_string()).or_else(|| {
        ha_response.error_type.as_ref().map(|et| {
            if let Some(ref origin) = ha_response.error_origin {
                format!("{}: {}", et, origin)
            } else {
                et.clone()
            }
        })
    });

    // Check if record exists
    let existing = get_result_by_package_file_sha256(conn, package_name, file_path, sha256)?;

    if let Some(existing_record) = existing {
        // Update existing record
        diesel::update(dsl::hybridanalysis_results.find(existing_record.id))
            .set((
                dsl::job_id.eq(&ha_response.job_id),
                dsl::environment_id.eq(ha_response.environment_id),
                dsl::environment_description.eq(&ha_response.environment_description),
                dsl::state.eq(&ha_response.state),
                dsl::verdict.eq(&ha_response.verdict),
                dsl::threat_score.eq(ha_response.threat_score),
                dsl::threat_level.eq(ha_response.threat_level),
                dsl::total_signatures.eq(ha_response.total_signatures),
                dsl::classification_tags.eq(&classification_tags_json),
                dsl::tags.eq(&tags_json),
                dsl::raw_response.eq(&raw_response),
                dsl::error_message.eq(&error_msg),
                dsl::updated_at.eq(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i32),
            ))
            .execute(conn)?;

        Ok(dsl::hybridanalysis_results
            .find(existing_record.id)
            .first(conn)?)
    } else {
        // Insert new record
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i32;

        let new_result = NewHybridAnalysisResult {
            package_name,
            file_path,
            sha256,
            job_id: &ha_response.job_id,
            environment_id: ha_response.environment_id,
            environment_description: &ha_response.environment_description,
            state: &ha_response.state,
            verdict: &ha_response.verdict,
            threat_score: ha_response.threat_score,
            threat_level: ha_response.threat_level,
            total_signatures: ha_response.total_signatures,
            classification_tags: &classification_tags_json,
            tags: &tags_json,
            raw_response: &raw_response,
            created_at: current_time,
            updated_at: current_time,
            error_message: error_msg.as_deref(),
        };

        diesel::insert_into(hybridanalysis_results::table)
            .values(&new_result)
            .execute(conn)?;

        // Get the last inserted record
        Ok(dsl::hybridanalysis_results
            .order(dsl::id.desc())
            .first(conn)?)
    }
}

/// Synchronous upsert (for immediate operations)
pub fn upsert_result(
    conn: &mut SqliteConnection,
    package_name: &str,
    file_path: &str,
    sha256: &str,
    ha_response: &HybridAnalysisReportResponse,
) -> Result<HybridAnalysisResult, Box<dyn Error>> {
    upsert_result_internal(conn, package_name, file_path, sha256, ha_response)
}

/// Save an error result to the database
pub fn save_error_result(
    conn: &mut SqliteConnection,
    package_name: &str,
    file_path: &str,
    sha256: &str,
    verdict: &str,
    error_message: &str,
) -> Result<HybridAnalysisResult, Box<dyn Error>> {
    use crate::schema::hybridanalysis_results::dsl;

    // Check if record exists
    let existing = get_result_by_package_file_sha256(conn, package_name, file_path, sha256)?;

    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32;

    if let Some(existing_record) = existing {
        // Update existing record with error
        diesel::update(dsl::hybridanalysis_results.find(existing_record.id))
            .set((
                dsl::verdict.eq(verdict),
                dsl::error_message.eq(Some(error_message)),
                dsl::updated_at.eq(current_time),
            ))
            .execute(conn)?;

        Ok(dsl::hybridanalysis_results
            .find(existing_record.id)
            .first(conn)?)
    } else {
        // Insert new error record
        let new_result = NewHybridAnalysisResult {
            package_name,
            file_path,
            sha256,
            job_id: "",
            environment_id: 0,
            environment_description: "",
            state: "error",
            verdict,
            threat_score: None,
            threat_level: None,
            total_signatures: None,
            classification_tags: "[]",
            tags: "[]",
            raw_response: "{}",
            created_at: current_time,
            updated_at: current_time,
            error_message: Some(error_message),
        };

        diesel::insert_into(hybridanalysis_results::table)
            .values(&new_result)
            .execute(conn)?;

        Ok(dsl::hybridanalysis_results
            .order(dsl::id.desc())
            .first(conn)?)
    }
}

/// Delete all Hybrid Analysis results for a package by package name
pub fn delete_results_by_package(
    conn: &mut SqliteConnection,
    package_name: &str,
) -> Result<usize, Box<dyn Error>> {
    use crate::schema::hybridanalysis_results::dsl;

    let deleted =
        diesel::delete(dsl::hybridanalysis_results.filter(dsl::package_name.eq(package_name)))
            .execute(conn)?;

    Ok(deleted)
}
