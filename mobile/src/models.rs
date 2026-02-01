use super::schema::posts;
use diesel::prelude::*;
use serde::Serialize;

#[allow(dead_code)]
#[derive(Queryable, Selectable, Serialize)]
#[diesel(table_name = posts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}

#[derive(Insertable)]
#[diesel(table_name = posts)]
pub struct NewPost<'a> {
    pub title: &'a str,
    pub body: &'a str,
}

// google play app api cache
#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = super::schema::google_play_apps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct GooglePlayApp {
    pub id: i32,
    pub package_id: String,
    pub title: String,
    pub developer: String,
    pub version: Option<String>,
    pub icon_base64: Option<String>,
    pub score: Option<f32>,
    pub installs: Option<String>,
    pub updated: Option<i32>,
    pub raw_response: String,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = super::schema::google_play_apps)]
pub struct NewGooglePlayApp<'a> {
    pub package_id: &'a str,
    pub title: &'a str,
    pub developer: &'a str,
    pub version: Option<&'a str>,
    pub icon_base64: Option<&'a str>,
    pub score: Option<f32>,
    pub installs: Option<&'a str>,
    pub updated: Option<i32>,
    pub raw_response: &'a str,
    pub created_at: i32,
    pub updated_at: i32,
}

// virustotal app api cache
#[derive(Queryable, Selectable, Serialize, Clone)]
#[diesel(table_name = super::schema::virustotal_results)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct VirusTotalResult {
    pub id: i32,
    pub package_name: String,
    pub file_path: String,
    pub sha256: String,
    pub last_analysis_date: i32,
    pub malicious: i32,
    pub suspicious: i32,
    pub undetected: i32,
    pub harmless: i32,
    pub timeout: i32,
    pub failure: i32,
    pub type_unsupported: i32,
    pub dex_count: Option<i32>,
    pub reputation: i32,
    pub raw_response: String,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = super::schema::virustotal_results)]
pub struct NewVirusTotalResult<'a> {
    pub package_name: &'a str,
    pub file_path: &'a str,
    pub sha256: &'a str,
    pub last_analysis_date: i32,
    pub malicious: i32,
    pub suspicious: i32,
    pub undetected: i32,
    pub harmless: i32,
    pub timeout: i32,
    pub failure: i32,
    pub type_unsupported: i32,
    pub dex_count: Option<i32>,
    pub reputation: i32,
    pub raw_response: &'a str,
    pub created_at: i32,
    pub updated_at: i32,
}

// hybridanalysis app api cache
#[derive(Queryable, Selectable, Serialize, Clone)]
#[diesel(table_name = super::schema::hybridanalysis_results)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct HybridAnalysisResult {
    pub id: i32,
    pub package_name: String,
    pub file_path: String,
    pub sha256: String,
    pub job_id: String,
    pub environment_id: i32,
    pub environment_description: String,
    pub state: String,
    pub verdict: String,
    pub threat_score: Option<i32>,
    pub threat_level: Option<i32>,
    pub total_signatures: Option<i32>,
    pub classification_tags: String,
    pub tags: String,
    pub raw_response: String,
    pub created_at: i32,
    pub updated_at: i32,
    pub error_message: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = super::schema::hybridanalysis_results)]
pub struct NewHybridAnalysisResult<'a> {
    pub package_name: &'a str,
    pub file_path: &'a str,
    pub sha256: &'a str,
    pub job_id: &'a str,
    pub environment_id: i32,
    pub environment_description: &'a str,
    pub state: &'a str,
    pub verdict: &'a str,
    pub threat_score: Option<i32>,
    pub threat_level: Option<i32>,
    pub total_signatures: Option<i32>,
    pub classification_tags: &'a str,
    pub tags: &'a str,
    pub raw_response: &'a str,
    pub created_at: i32,
    pub updated_at: i32,
    pub error_message: Option<&'a str>,
}

// package info cache
#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = super::schema::package_info_cache)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct PackageInfoCache {
    pub id: i32,
    pub pkg_id: String,
    pub pkg_checksum: String,
    pub dump_text: String,
    pub code_path: String,
    pub version_code: i32,
    pub version_name: String,
    pub first_install_time: String,
    pub last_update_time: String,
    pub apk_path: Option<String>,
    pub apk_sha256sum: Option<String>,
    pub izzyscore: Option<i32>,
    pub device_serial: String,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = super::schema::package_info_cache)]
pub struct NewPackageInfoCache<'a> {
    pub pkg_id: &'a str,
    pub pkg_checksum: &'a str,
    pub dump_text: &'a str,
    pub code_path: &'a str,
    pub version_code: i32,
    pub version_name: &'a str,
    pub first_install_time: &'a str,
    pub last_update_time: &'a str,
    pub apk_path: Option<&'a str>,
    pub apk_sha256sum: Option<&'a str>,
    pub izzyscore: Option<i32>,
    pub device_serial: &'a str,
    pub created_at: i32,
    pub updated_at: i32,
}

// fdroid app api cache
#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = super::schema::fdroid_apps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct FDroidApp {
    pub id: i32,
    pub package_id: String,
    pub title: String,
    pub developer: String,
    pub version: Option<String>,
    pub icon_base64: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub updated: Option<i32>,
    pub raw_response: String,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = super::schema::fdroid_apps)]
pub struct NewFDroidApp<'a> {
    pub package_id: &'a str,
    pub title: &'a str,
    pub developer: &'a str,
    pub version: Option<&'a str>,
    pub icon_base64: Option<&'a str>,
    pub description: Option<&'a str>,
    pub license: Option<&'a str>,
    pub updated: Option<i32>,
    pub raw_response: &'a str,
    pub created_at: i32,
    pub updated_at: i32,
}

// apkmirror app api cache
#[derive(Queryable, Selectable, Serialize, Clone, Debug)]
#[diesel(table_name = super::schema::apkmirror_apps)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ApkMirrorApp {
    pub id: i32,
    pub package_id: String,
    pub title: String,
    pub developer: String,
    pub version: Option<String>,
    pub icon_url: Option<String>,
    pub icon_base64: Option<String>,
    pub raw_response: String,
    pub created_at: i32,
    pub updated_at: i32,
}

#[derive(Insertable)]
#[diesel(table_name = super::schema::apkmirror_apps)]
pub struct NewApkMirrorApp<'a> {
    pub package_id: &'a str,
    pub title: &'a str,
    pub developer: &'a str,
    pub version: Option<&'a str>,
    pub icon_url: Option<&'a str>,
    pub icon_base64: Option<&'a str>,
    pub raw_response: &'a str,
    pub created_at: i32,
    pub updated_at: i32,
}
