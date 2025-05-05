//! Search at `/search`.
use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use crate::serve::ServerContext;
use crate::serve::content_type;
use crate::serve::is_logged_in;
use crate::serve::not_found;
use crate::serve::response;
use axum::Router;
use axum::body::Body;
use axum::extract::Form;
use axum::extract::Multipart;
use axum::extract::Path;
use axum::extract::Query;
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
pub struct SearchForm {
    pub q: Option<String>,
}

fn search_form() -> &'static str {
    "
    <form action='/search' method='get'>
        <input type='text' name='q' />
        <button type='submit'>Search</button>
    </form>
    "
}

async fn search_page(ctx: &ServerContext, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/html");
    let title = "Search";
    let extra_head = "";
    let settings = PageSettings::new(&title, is_logged_in, false, Top::GoHome, extra_head);
    let body = search_form();
    let body = page(&ctx, &settings, &body).await;
    response(StatusCode::OK, headers, body, &ctx)
}

struct SearchResult {
    id: i64,
    content: String,
}

async fn search_results(ctx: &ServerContext, q: &str) -> Vec<SearchResult> {
    let conn = ctx.conn().await;
    let mut results = conn
        .prepare("SELECT * FROM posts WHERE content LIKE ?")
        .unwrap();
    let results = results
        .query_map([q], |row| {
            let content: String = row.get("content")?;
            let id: i64 = row.get("id")?;
            let post = SearchResult { id, content };
            Ok(post)
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    results
}

async fn get_search(
    State(ctx): State<ServerContext>,
    jar: CookieJar,
    search: Query<SearchForm>,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let search_form = search_form();
    let q = search.q.clone().unwrap_or_default();
    let results = search_results(&ctx, &q).await;
    let results = results
        .iter()
        .map(|r| {
            format!(
                "
    <div>
        <h2>{}</h2>
        <p>{}</p>
    </div>
    ",
                r.id, r.content
            )
        })
        .collect::<Vec<_>>();
    let results = results.join("\n");
    let body = format!(
        "
    {search_form}
    <p>Query: {}
    {results}
    ",
        q
    );
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/html");
    let title = "Search";
    let extra_head = "";
    let settings = PageSettings::new(&title, is_logged_in, false, Top::GoHome, extra_head);
    let body = page(&ctx, &settings, &body).await;
    response(StatusCode::OK, headers, body, &ctx)
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router.clone().route("/search", get(get_search))
}
