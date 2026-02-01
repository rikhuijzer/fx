//! Search at `/search`.
use crate::data::Kv;
use crate::data::Post;
use crate::data::SqliteDateTime;
use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use crate::html::wrap_post_content;
use crate::serve::ServerContext;
use crate::serve::content_type;
use crate::serve::is_logged_in;
use crate::serve::response;
use axum::Router;
use axum::body::Body;
use axum::extract::Query;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::Response;
use axum::http::StatusCode;
use axum::routing::get;
use axum_extra::extract::CookieJar;
use rusqlite::Result;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchForm {
    pub q: Option<String>,
}

fn search_form(q: &str) -> String {
    let q_escaped = crate::html::escape_html(q);
    format!(
        "
        <form action='/search' method='get'>
            <input type='text' name='q' value='{q_escaped}' />
            <button type='submit'>Search</button>
        </form>
        "
    )
}

async fn search(ctx: &ServerContext, q: &str) -> Vec<Post> {
    if q.is_empty() {
        return vec![];
    }
    let conn = ctx.conn().await;

    // Use a closure to handle errors and ensure rollback
    let search_result = (|| -> Result<Vec<Post>, rusqlite::Error> {
        conn.execute("BEGIN TRANSACTION", [])?;

        // Creating a virtual table on each query to avoid having to manually keep
        // track of updates to the fts table. For small sites, creating the index on
        // each query should be fine.
        conn.execute(
            "CREATE VIRTUAL TABLE posts_fts USING fts5(
                id,
                created,
                updated,
                content,
                content=posts,
                tokenize=trigram
            )",
            [],
        )?;

        conn.execute(
            "INSERT INTO posts_fts (id, created, updated, content)
             SELECT id, created, updated, content FROM posts",
            [],
        )?;

        let mut stmt = conn.prepare("SELECT * FROM posts_fts WHERE posts_fts MATCH ?")?;
        let results = stmt
            .query_map([q], |row| {
                let id: i64 = row.get("id")?;
                let created: String = row.get("created")?;
                let updated: String = row.get("updated")?;
                let content: String = row.get("content")?;
                let post = Post {
                    id,
                    created: SqliteDateTime::from_sqlite(&created),
                    updated: SqliteDateTime::from_sqlite(&updated),
                    content,
                };
                Ok(post)
            })?
            .collect::<Result<Vec<_>, _>>()?;

        conn.execute("ROLLBACK", [])?;
        Ok(results)
    })();

    match search_result {
        Ok(results) => results,
        Err(e) => {
            tracing::error!("Search failed: {e}");
            // Attempt to rollback in case of error
            let _ = conn.execute("ROLLBACK", []);
            vec![]
        }
    }
}

async fn get_search(
    State(ctx): State<ServerContext>,
    jar: CookieJar,
    search_query: Query<SearchForm>,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let q = search_query.q.clone().unwrap_or_default();
    let search_form = search_form(&q);
    let mut results = search(&ctx, &q).await;
    let results = results
        .iter_mut()
        .map(|p| {
            let slug = crate::md::extract_slug(p);
            crate::md::preview(p, 60);
            let is_front_page_preview = true;
            wrap_post_content(p, &slug, is_front_page_preview)
        })
        .collect::<Vec<_>>();
    let results = results.join("\n");
    let body = format!(
        "
        {search_form}
        {results}
        "
    );
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/html");
    let title = "Search";
    let extra_head = &Kv::get_or_empty_string(&*ctx.conn().await, "extra_head");
    let settings = PageSettings::new(title, Some(is_logged_in), false, Top::GoHome, extra_head);
    let body = page(&ctx, &settings, &body).await;
    response(StatusCode::OK, headers, body, &ctx)
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router.clone().route("/search", get(get_search))
}
