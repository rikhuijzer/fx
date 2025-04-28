use crate::data::Kv;
use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use crate::serve::ServerContext;
use crate::serve::is_logged_in;
use crate::serve::response;
use axum::Form;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::Response;
use axum::http::StatusCode;
use axum::routing::get;
use axum::routing::post;
use axum_extra::extract::CookieJar;
use rusqlite::Connection;
use rusqlite::Result;
use rusqlite::params;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
pub struct File {
    // The sha of the file. This is the primary key. The benefit of this is that
    // we can accept any uploads via the HTML file input since duplicates are
    // simply the same file. Another benefit is that it will work well with
    // having hidden posts later. Thanks to it being a sha, it will be hard to
    // guess the name of the file.
    pub sha: String,
    pub mime_type: String,
    /// Filename is shown for easier identification.
    pub filename: String,
    pub data: Vec<u8>,
}

impl File {
    pub fn create_table(conn: &Connection) -> Result<usize> {
        let stmt = "
            CREATE TABLE IF NOT EXISTS files (
                name TEXT PRIMARY KEY,
                mime_type TEXT NOT NULL,
                filename TEXT NOT NULL,
                data BLOB NOT NULL
            );
        ";
        conn.execute(stmt, [])
    }
    pub fn list(conn: &Connection) -> rusqlite::Result<Vec<Self>> {
        let stmt = "
            SELECT sha, mime_type, filename, data
            FROM files;
            ";
        let mut stmt = conn.prepare(stmt)?;
        let files = stmt.query_map([], |row| {
            Ok(File {
                sha: row.get("sha")?,
                mime_type: row.get("mime_type")?,
                filename: row.get("filename")?,
                data: row.get("data")?,
            })
        })?;
        Ok(files.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn insert(conn: &Connection, file: &Self) -> rusqlite::Result<usize> {
        let sql = "
            INSERT INTO files (sha, mime_type, filename, data)
            VALUES (?, ?, ?, ?);
            ";
        let params = params![file.sha, file.mime_type, file.filename, file.data];
        conn.execute(sql, params)
    }
    pub fn get(conn: &Connection, name: &str) -> rusqlite::Result<Self> {
        let stmt = "
            SELECT sha, mime_type, filename, data
            FROM files
            WHERE name = ?;
            ";
        let mut stmt = conn.prepare(stmt)?;
        let file = stmt.query_row([name], |row| {
            Ok(File {
                sha: row.get("sha")?,
                mime_type: row.get("mime_type")?,
                filename: row.get("filename")?,
                data: row.get("data")?,
            })
        });
        let file = match file {
            Ok(file) => file,
            Err(e) => {
                let msg = "Could not get file from database";
                tracing::error!("{msg}: {e}");
                return Err(e);
            }
        };
        Ok(file)
    }
}

async fn get_files(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx);
    }
    let body = "foo";
    let page_settings = PageSettings::new("Files", is_logged_in, false, Top::GoHome, "");
    let body = page(&ctx, &page_settings, &body);
    response(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

async fn post_file(
    State(ctx): State<ServerContext>,
    jar: CookieJar,
    Form(_form): Form<File>,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx);
    }
    // let conn = &ctx.conn_lock();
    todo!()
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router
        .clone()
        .route("/files", get(get_files))
        .route("/file/add", post(post_file))
}
