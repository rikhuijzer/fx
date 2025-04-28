use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use crate::serve::ServerContext;
use crate::serve::is_logged_in;
use crate::serve::not_found;
use crate::serve::response;
use axum::Router;
use axum::body::Body;
use axum::extract::Form;
use axum::extract::Multipart;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::Response;
use axum::http::StatusCode;
use axum::routing::get;
use axum::routing::post;
use axum_extra::extract::CookieJar;
use bytes::Bytes;
use rusqlite::Connection;
use rusqlite::Result;
use rusqlite::params;
use serde::Deserialize;
use serde::Serialize;
use sha2::Digest;

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
    pub data: Bytes,
}

fn bytes_to_blob(bytes: &Bytes) -> Vec<u8> {
    bytes.to_vec()
}

fn blob_to_bytes(blob: Vec<u8>) -> Bytes {
    Bytes::from(blob)
}

impl File {
    pub fn create_table(conn: &Connection) -> Result<usize> {
        let stmt = "
            CREATE TABLE IF NOT EXISTS files (
                sha TEXT PRIMARY KEY,
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
            FROM files
            ORDER BY filename;
            ";
        let mut stmt = conn.prepare(stmt)?;
        let files = stmt.query_map([], |row| {
            Ok(File {
                sha: row.get("sha")?,
                mime_type: row.get("mime_type")?,
                filename: row.get("filename")?,
                data: blob_to_bytes(row.get("data")?),
            })
        })?;
        files.collect::<Result<Vec<_>, _>>()
    }
    pub fn insert(conn: &Connection, file: &Self) -> rusqlite::Result<usize> {
        // We can safely use `INSERT OR REPLACE` because the sha is the primary
        // key. In the "worse" case, it will only rename the filename. If the
        // mime type is different, then it means that the extension was changed
        // locally which probably is good to reflect in the database. Also
        // having the need for two identical files with different mimetypes is a
        // very unlikely scenario.
        let sql = "
            INSERT OR REPLACE INTO files (sha, mime_type, filename, data)
            VALUES (?, ?, ?, ?);
            ";
        let data = bytes_to_blob(&file.data);
        let params = params![file.sha, file.mime_type, file.filename, data];
        conn.execute(sql, params)
    }
    pub fn get(conn: &Connection, name: &str) -> rusqlite::Result<Self> {
        let stmt = "
            SELECT sha, mime_type, filename, data
            FROM files
            WHERE sha = ?;
            ";
        let mut stmt = conn.prepare(stmt)?;
        let file = stmt.query_row([name], |row| {
            Ok(File {
                sha: row.get("sha")?,
                mime_type: row.get("mime_type")?,
                filename: row.get("filename")?,
                data: blob_to_bytes(row.get("data")?),
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
    pub fn delete(conn: &Connection, sha: &str) -> rusqlite::Result<usize> {
        let sql = "DELETE FROM files WHERE sha = ?";
        conn.execute(sql, [sha])
    }
    pub fn rename(conn: &Connection, sha: &str, filename: &str) -> rusqlite::Result<usize> {
        let sql = "UPDATE files SET filename = ? WHERE sha = ?";
        conn.execute(sql, [filename, sha])
    }
}

fn md_link(file: &File) -> String {
    if file.mime_type.starts_with("image/") {
        format!("![{}](/files/{})", file.filename, file.sha)
    } else {
        format!("[{}](/files/{})", file.filename, file.sha)
    }
}

fn show_file(file: &File) -> String {
    let sha = &file.sha;
    format!(
        "
        <div style='padding: 10px; padding-bottom: 0px; padding-top: 16px; \
          border-bottom: 1px solid var(--border);'>
            <a href='/files/{sha}'>{}</a>&nbsp;&nbsp;
            <a class='unstyled-link' href='/files/rename/{sha}' \
              style='font-size: 0.8rem; padding-top: 0.1rem;'>
                ✏️ Rename
            </a>&nbsp;
            <a class='unstyled-link' href='/files/delete/{sha}' \
              style='font-size: 0.8rem; padding-top: 0.1rem;'>
                🗑️ Delete
            </a><br>
            <span style='font-size: var(--ui-font-size);'>
                Markdown link:
            </span><br>
            <pre style='margin-top: 6px; margin-bottom: 0px;'>
                <code class='language-md'>{}</code>
            </pre>
        </div>
        ",
        file.filename,
        md_link(file)
    )
}

async fn get_files(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx);
    }
    let files = File::list(&ctx.conn_lock()).unwrap();
    let files = files
        .iter()
        .map(show_file)
        .collect::<Vec<String>>()
        .join("");
    let body = format!(
        "
        <div>
            <form method='post' action='/files/add' \
              enctype='multipart/form-data' \
              style='padding: 10px; text-align: center; \
              border-bottom: 2px solid var(--border);'>
                <div>
                    <label for='file'>Choose file(s) to upload (max 15MB)</label>
                    <input type='file' id='file' name='file' multiple />
                </div>
                <div>
                    <button>Upload</button>
                </div>
            </form>
        </div>
        <div>
            {files}
        </div>
        "
    );
    let page_settings = PageSettings::new("Files", is_logged_in, false, Top::GoHome, "");
    let body = page(&ctx, &page_settings, &body);
    response(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

async fn get_file(State(ctx): State<ServerContext>, Path(sha): Path<String>) -> Response<Body> {
    let file = File::get(&ctx.conn_lock(), &sha).unwrap();
    let mut headers = HeaderMap::new();
    crate::serve::content_type(&mut headers, &file.mime_type);
    // Setting this too high might make deleted files accessible for too long
    // which could be confusing for the author.
    let max_age = 60;
    crate::serve::enable_caching(&mut headers, max_age);
    response(StatusCode::OK, headers, file.data, &ctx)
}

async fn post_file(
    State(ctx): State<ServerContext>,
    jar: CookieJar,
    mut multipart: Multipart,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx);
    }
    while let Some(field) = multipart.next_field().await.unwrap() {
        let filename = field.file_name().unwrap().to_string();
        if filename.is_empty() {
            // This occurs when clicking "Upload" without selecting any files.
            continue;
        }
        let mime_type = field.content_type().unwrap().to_string();
        let data = field
            .bytes()
            .await
            .expect("Failed to read file; the file could be too large.");
        let sha = sha2::Sha256::digest(&data);
        let sha = hex::encode(sha);
        let file = File {
            sha,
            mime_type,
            filename,
            data,
        };
        File::insert(&ctx.conn_lock(), &file).unwrap();
    }
    crate::serve::see_other(&ctx, "/files")
}

async fn get_delete(
    State(ctx): State<ServerContext>,
    Path(sha): Path<String>,
    jar: CookieJar,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx);
    }
    let file = File::get(&ctx.conn_lock(), &sha);
    let file = match file {
        Ok(file) => file,
        Err(_) => return not_found(State(ctx.clone())).await,
    };
    let extra_head = &ctx.args.extra_head;
    let title = format!("Delete: {}", file.filename);
    let settings = PageSettings::new(&title, false, false, Top::GoHome, extra_head);
    let body = indoc::formatdoc! {r#"
        <div class='medium-text' style='text-align: center; font-weight: bold;'>
            <p>Are you sure you want to delete <code>{}</code>? This action cannot be undone.</p>
            <form action='/files/delete/{sha}' method='post'>
                <button type='submit'>Delete</button>
            </form>
            <br>
        </div>
    "#, file.filename};
    let body = page(&ctx, &settings, &body);
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

async fn post_delete(
    State(ctx): State<ServerContext>,
    Path(sha): Path<String>,
    jar: CookieJar,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx);
    }
    File::delete(&ctx.conn_lock(), &sha).unwrap();
    crate::serve::see_other(&ctx, "/files")
}

async fn get_rename(
    State(ctx): State<ServerContext>,
    Path(sha): Path<String>,
    jar: CookieJar,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx);
    }
    let file = File::get(&ctx.conn_lock(), &sha);
    let file = match file {
        Ok(file) => file,
        Err(_) => return not_found(State(ctx.clone())).await,
    };
    let extra_head = &ctx.args.extra_head;
    let title = format!("Rename: {}", file.filename);
    let settings = PageSettings::new(&title, false, false, Top::GoHome, extra_head);
    let body = indoc::formatdoc! {r#"
        <div class='medium-text' style='text-align: center;'>
            <p>Rename file: <code>{}</code></p>
            <form action='/files/rename/{sha}' method='post'>
                <div>
                    <label for='filename'>New filename:</label>
                    <input type='text' id='filename' name='filename' value='{}' />
                </div>
                <div style='margin-top: 10px;'>
                    <button type='submit'>Change</button>
                </div>
            </form>
            <br>
        </div>
    "#, file.filename, file.filename};
    let body = page(&ctx, &settings, &body);
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

#[derive(Debug, Deserialize)]
struct RenameForm {
    filename: String,
}

async fn post_rename(
    State(ctx): State<ServerContext>,
    Path(sha): Path<String>,
    jar: CookieJar,
    Form(rename_form): Form<RenameForm>,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx);
    }
    let filename = rename_form.filename;
    File::rename(&ctx.conn_lock(), &sha, &filename).unwrap();
    crate::serve::see_other(&ctx, "/files")
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router
        .clone()
        .route("/files", get(get_files))
        .route("/files/{sha}", get(get_file))
        .route("/files/add", post(post_file))
        .route("/files/delete/{sha}", get(get_delete))
        .route("/files/delete/{sha}", post(post_delete))
        .route("/files/rename/{sha}", get(get_rename))
        .route("/files/rename/{sha}", post(post_rename))
}
