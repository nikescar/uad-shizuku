use std::sync::Mutex;
use std::sync::Once;

use diesel::prelude::*;
use wasm_bindgen::prelude::*;

#[cfg(not(target_family = "wasm"))]
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

#[cfg(not(target_family = "wasm"))]
const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[wasm_bindgen]
extern "C" {
    // Use `js_namespace` here to bind `console.log(..)` instead of just
    // `log(..)`
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

// Next let's define a macro that's like `println!`, only it works for
// `console.log`. Note that `println!` doesn't actually work on the Wasm target
// because the standard library currently just eats all output. To get
// `println!`-like behavior in your app you'll likely want a macro like this.
#[allow(unused_macros)]
macro_rules! console_log {
    // Note that this is using the `log` function imported above during
    // `bare_bones`
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

static VFS: Mutex<(i32, Once)> = Mutex::new((0, Once::new()));
static DB_PATH: Mutex<Option<String>> = Mutex::new(None);
static MIGRATIONS_RAN: Mutex<bool> = Mutex::new(false);

/// Set the database path to use for connections
pub fn set_db_path(path: String) {
    let mut db_path = DB_PATH.lock().unwrap();
    *db_path = Some(path);
    // Reset migrations flag when database path changes
    let mut migrations_ran = MIGRATIONS_RAN.lock().unwrap();
    *migrations_ran = false;
}

pub fn establish_connection() -> SqliteConnection {
    let (vfs, _once) = &*VFS.lock().unwrap();

    // Get the database path from the static or use default
    let db_path = DB_PATH.lock().unwrap();
    let base_path = db_path.as_deref().unwrap_or("uad.db");

    let url = match vfs {
        0 => base_path.to_string(),
        1 => format!("file:{}?vfs=opfs-sahpool", base_path),
        2 => format!("file:{}?vfs=relaxed-idb", base_path),
        _ => unreachable!(),
    };
    let mut conn = SqliteConnection::establish(&url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", base_path));

    #[cfg(not(target_family = "wasm"))]
    {
        // Enable WAL mode for better concurrent access
        diesel::sql_query("PRAGMA journal_mode=WAL;")
            .execute(&mut conn)
            .ok();

        // Set busy timeout to wait up to 30 seconds when database is locked
        // This prevents "database is locked" errors during concurrent access
        diesel::sql_query("PRAGMA busy_timeout=30000;")
            .execute(&mut conn)
            .ok();

        // Run migrations only once per database
        let mut migrations_ran = MIGRATIONS_RAN.lock().unwrap();
        if !*migrations_ran {
            conn.run_pending_migrations(MIGRATIONS)
                .expect("Failed to run database migrations");
            *migrations_ran = true;
        }
    }

    conn
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
#[wasm_bindgen(js_name = installOpfsSahpool)]
pub async fn install_opfs_sahpool() {
    use sqlite_wasm_vfs::sahpool::{install, OpfsSAHPoolCfg};
    install(&OpfsSAHPoolCfg::default(), false).await.unwrap();
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
#[wasm_bindgen(js_name = installRelaxedIdb)]
pub async fn install_relaxed_idb() {
    use sqlite_wasm_vfs::relaxed_idb::{install, RelaxedIdbCfg};
    install(&RelaxedIdbCfg::default(), false).await.unwrap();
}

#[wasm_bindgen(js_name = switchVfs)]
pub fn switch_vfs(id: i32) {
    *VFS.lock().unwrap() = (id, Once::new());
}

pub fn invalidate_cache() {
    let connection = &mut establish_connection();

    let queries = [
        "DELETE FROM apkmirror_apps WHERE package_id = title",
        "DELETE FROM fdroid_apps WHERE title = \"Not Found\"",
        "DELETE FROM google_play_apps WHERE title = \"Not Found\"",
        "DELETE FROM hybridanalysis_results WHERE state = \"not_found\"",
    ];

    for query in queries {
        match diesel::sql_query(query).execute(connection) {
            Ok(count) => {
                #[cfg(not(target_family = "wasm"))]
                log::info!("Executed query: '{}'. Deleted {} rows.", query, count);
                #[cfg(target_family = "wasm")]
                console_log!("Executed query: '{}'. Deleted {} rows.", query, count);
            }
            Err(e) => {
                #[cfg(not(target_family = "wasm"))]
                log::error!("Failed to execute query: '{}'. Error: {}", query, e);
                #[cfg(target_family = "wasm")]
                console_log!("Failed to execute query: '{}'. Error: {}", query, e);
            }
        }
    }
}

/// Flush (delete all records from) the virustotal_results table
pub fn flush_virustotal() {
    let connection = &mut establish_connection();
    match diesel::sql_query("DELETE FROM virustotal_results").execute(connection) {
        Ok(count) => {
            #[cfg(not(target_family = "wasm"))]
            log::info!("Flushed virustotal_results table. Deleted {} rows.", count);
            #[cfg(target_family = "wasm")]
            console_log!("Flushed virustotal_results table. Deleted {} rows.", count);
        }
        Err(e) => {
            #[cfg(not(target_family = "wasm"))]
            log::error!("Failed to flush virustotal_results table: {}", e);
            #[cfg(target_family = "wasm")]
            console_log!("Failed to flush virustotal_results table: {}", e);
        }
    }
}

/// Flush (delete all records from) the hybridanalysis_results table
pub fn flush_hybridanalysis() {
    let connection = &mut establish_connection();
    match diesel::sql_query("DELETE FROM hybridanalysis_results").execute(connection) {
        Ok(count) => {
            #[cfg(not(target_family = "wasm"))]
            log::info!("Flushed hybridanalysis_results table. Deleted {} rows.", count);
            #[cfg(target_family = "wasm")]
            console_log!("Flushed hybridanalysis_results table. Deleted {} rows.", count);
        }
        Err(e) => {
            #[cfg(not(target_family = "wasm"))]
            log::error!("Failed to flush hybridanalysis_results table: {}", e);
            #[cfg(target_family = "wasm")]
            console_log!("Failed to flush hybridanalysis_results table: {}", e);
        }
    }
}

/// Flush (delete all records from) the google_play_apps table
pub fn flush_googleplay() {
    let connection = &mut establish_connection();
    match diesel::sql_query("DELETE FROM google_play_apps").execute(connection) {
        Ok(count) => {
            #[cfg(not(target_family = "wasm"))]
            log::info!("Flushed google_play_apps table. Deleted {} rows.", count);
            #[cfg(target_family = "wasm")]
            console_log!("Flushed google_play_apps table. Deleted {} rows.", count);
        }
        Err(e) => {
            #[cfg(not(target_family = "wasm"))]
            log::error!("Failed to flush google_play_apps table: {}", e);
            #[cfg(target_family = "wasm")]
            console_log!("Failed to flush google_play_apps table: {}", e);
        }
    }
}

/// Flush (delete all records from) the fdroid_apps table
pub fn flush_fdroid() {
    let connection = &mut establish_connection();
    match diesel::sql_query("DELETE FROM fdroid_apps").execute(connection) {
        Ok(count) => {
            #[cfg(not(target_family = "wasm"))]
            log::info!("Flushed fdroid_apps table. Deleted {} rows.", count);
            #[cfg(target_family = "wasm")]
            console_log!("Flushed fdroid_apps table. Deleted {} rows.", count);
        }
        Err(e) => {
            #[cfg(not(target_family = "wasm"))]
            log::error!("Failed to flush fdroid_apps table: {}", e);
            #[cfg(target_family = "wasm")]
            console_log!("Failed to flush fdroid_apps table: {}", e);
        }
    }
}

/// Flush (delete all records from) the apkmirror_apps table
pub fn flush_apkmirror() {
    let connection = &mut establish_connection();
    match diesel::sql_query("DELETE FROM apkmirror_apps").execute(connection) {
        Ok(count) => {
            #[cfg(not(target_family = "wasm"))]
            log::info!("Flushed apkmirror_apps table. Deleted {} rows.", count);
            #[cfg(target_family = "wasm")]
            console_log!("Flushed apkmirror_apps table. Deleted {} rows.", count);
        }
        Err(e) => {
            #[cfg(not(target_family = "wasm"))]
            log::error!("Failed to flush apkmirror_apps table: {}", e);
            #[cfg(target_family = "wasm")]
            console_log!("Failed to flush apkmirror_apps table: {}", e);
        }
    }
}

// #[wasm_bindgen(js_name = createPost)]
// pub fn create_post(title: &str, body: &str) -> JsValue {
//     use crate::schema::posts;

//     let new_post = NewPost { title, body };

//     let post = diesel::insert_into(posts::table)
//         .values(&new_post)
//         .returning(Post::as_returning())
//         .get_result(&mut establish_connection())
//         .expect("Error saving new post");

//     serde_wasm_bindgen::to_value(&post).unwrap()
// }

// #[wasm_bindgen(js_name = deletePost)]
// pub fn delete_post(pattern: &str) {
//     let connection = &mut establish_connection();
//     let num_deleted = diesel::delete(
//         schema::posts::dsl::posts.filter(schema::posts::title.like(pattern.to_string())),
//     )
//     .execute(connection)
//     .expect("Error deleting posts");

//     console_log!("Deleted {num_deleted} posts");
// }

// #[wasm_bindgen(js_name = getPost)]
// pub fn get_post(post_id: i32) -> JsValue {
//     use schema::posts::dsl::posts;

//     let connection = &mut establish_connection();

//     let post = posts
//         .find(post_id)
//         .select(Post::as_select())
//         .first(connection)
//         .optional(); // This allows for returning an Option<Post>, otherwise it will throw an error

//     match &post {
//         Ok(Some(post)) => console_log!("Post with id: {} has a title: {}", post.id, post.title),
//         Ok(None) => console_log!("Unable to find post {}", post_id),
//         Err(_) => console_log!("An error occurred while fetching post {}", post_id),
//     }
//     serde_wasm_bindgen::to_value(&post.ok().flatten()).unwrap()
// }

// #[wasm_bindgen(js_name = publishPost)]
// pub fn publish_post(id: i32) {
//     let connection = &mut establish_connection();

//     let post = diesel::update(schema::posts::dsl::posts.find(id))
//         .set(schema::posts::dsl::published.eq(true))
//         .returning(Post::as_returning())
//         .get_result(connection)
//         .unwrap();

//     console_log!("Published post {}", post.title);
// }

// // Core function for showing posts (usable from both WASM and CLI)
// pub fn show_posts_core() -> Vec<Post> {
//     let connection = &mut establish_connection();
//     let results = schema::posts::dsl::posts
//         .filter(schema::posts::dsl::published.eq(true))
//         .limit(5)
//         .select(Post::as_select())
//         .load(connection)
//         .expect("Error loading posts");

//     results
// }

// #[wasm_bindgen(js_name = showPosts)]
// pub fn show_posts() -> Vec<JsValue> {
//     let results = show_posts_core();

//     console_log!("Displaying {} posts", results.len());
//     for post in &results {
//         console_log!("{}", post.title);
//         console_log!("----------\n");
//         console_log!("{}", post.body);
//     }

//     results
//         .into_iter()
//         .map(|x| serde_wasm_bindgen::to_value(&x).unwrap())
//         .collect()
// }
