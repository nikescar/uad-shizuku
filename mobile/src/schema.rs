// @generated automatically by Diesel CLI.

diesel::table! {
    posts (id) {
        id -> Integer,
        title -> Text,
        body -> Text,
        published -> Bool,
    }
}

diesel::table! {
    google_play_apps (id) {
        id -> Integer,
        package_id -> Text,
        title -> Text,
        developer -> Text,
        version -> Nullable<Text>,
        icon_base64 -> Nullable<Text>,
        score -> Nullable<Float>,
        installs -> Nullable<Text>,
        updated -> Nullable<Integer>,
        raw_response -> Text,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    virustotal_results (id) {
        id -> Integer,
        package_name -> Text,
        file_path -> Text,
        sha256 -> Text,
        last_analysis_date -> Integer,
        malicious -> Integer,
        suspicious -> Integer,
        undetected -> Integer,
        harmless -> Integer,
        timeout -> Integer,
        failure -> Integer,
        type_unsupported -> Integer,
        dex_count -> Nullable<Integer>,
        reputation -> Integer,
        raw_response -> Text,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    hybridanalysis_results (id) {
        id -> Integer,
        package_name -> Text,
        file_path -> Text,
        sha256 -> Text,
        job_id -> Text,
        environment_id -> Integer,
        environment_description -> Text,
        state -> Text,
        verdict -> Text,
        threat_score -> Nullable<Integer>,
        threat_level -> Nullable<Integer>,
        total_signatures -> Nullable<Integer>,
        classification_tags -> Text,
        tags -> Text,
        raw_response -> Text,
        created_at -> Integer,
        updated_at -> Integer,
        error_message -> Nullable<Text>,
    }
}

diesel::table! {
    package_info_cache (id) {
        id -> Integer,
        pkg_id -> Text,
        pkg_checksum -> Text,
        dump_text -> Text,
        code_path -> Text,
        version_code -> Integer,
        version_name -> Text,
        first_install_time -> Text,
        last_update_time -> Text,
        apk_path -> Nullable<Text>,
        apk_sha256sum -> Nullable<Text>,
        izzyscore -> Nullable<Integer>,
        device_serial -> Text,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    fdroid_apps (id) {
        id -> Integer,
        package_id -> Text,
        title -> Text,
        developer -> Text,
        version -> Nullable<Text>,
        icon_base64 -> Nullable<Text>,
        description -> Nullable<Text>,
        license -> Nullable<Text>,
        updated -> Nullable<Integer>,
        raw_response -> Text,
        created_at -> Integer,
        updated_at -> Integer,
    }
}

diesel::table! {
    apkmirror_apps (id) {
        id -> Integer,
        package_id -> Text,
        title -> Text,
        developer -> Text,
        version -> Nullable<Text>,
        icon_url -> Nullable<Text>,
        icon_base64 -> Nullable<Text>,
        raw_response -> Text,
        created_at -> Integer,
        updated_at -> Integer,
    }
}
