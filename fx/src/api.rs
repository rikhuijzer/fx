use crate::data::Post;
use crate::serve::ServerContext;
use crate::serve::response;
use crate::serve::response_json;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::Response;
use axum::http::StatusCode;
use axum::http::header::HeaderMap;
use axum::http::header::HeaderValue;
use axum::routing::get;
use axum_extra::extract::CookieJar;
use serde_json::json;

async fn get_api(State(ctx): State<ServerContext>) -> Response<Body> {
    let domain = &ctx.args.domain;
    let domain = if let Some(domain) = domain {
        domain
    } else {
        ""
    };
    let port = ctx.args.port;
    let domain = if domain == "localhost" {
        format!("http://localhost:{port}")
    } else {
        format!("https://{domain}")
    };
    let body = json!({
        "download_all_url": format!("{domain}/download/all"),
    })
    .to_string();
    response_json(StatusCode::OK, body, &ctx)
}

fn is_authenticated(ctx: &ServerContext, jar: &CookieJar) -> bool {
    let password = &ctx.args.password;
    let password = if let Some(password) = password {
        password
    } else {
        tracing::warn!("admin password not set");
        return false;
    };
    let cookie = if let Some(cookie) = jar.get("Authorization") {
        cookie
    } else {
        return false;
    };
    let parts = cookie
        .value()
        .split_ascii_whitespace()
        .collect::<Vec<&str>>();
    if !parts.len() == 2 {
        return false;
    }
    if parts[0] != "Bearer" {
        return false;
    }
    let token = parts[1];
    token == password
}

fn error(ctx: &ServerContext, status: StatusCode, message: &str) -> Response<Body> {
    let body = json!({
        "status": status.as_u16(),
        "message": message,
    })
    .to_string();
    response_json(status, body, &ctx)
}

fn unauthorized(ctx: &ServerContext) -> Response<Body> {
    error(ctx, StatusCode::UNAUTHORIZED, "unauthorized")
}

async fn get_download_all(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    if !is_authenticated(&ctx, &jar) {
        return unauthorized(&ctx);
    }
    let conn = ctx.conn_lock();
    let posts = Post::list(&conn);
    let posts = if let Ok(posts) = posts {
        posts
    } else {
        return error(
            &ctx,
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to list posts",
        );
    };
    let body: Vec<u8> = vec![];
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/octet-stream"),
    );
    response::<Vec<u8>>(StatusCode::OK, headers, body, &ctx)
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router
        .clone()
        .route("/api", get(get_api))
        .route("/api/download/all", get(get_download_all))
}
