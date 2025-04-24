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

fn is_authenticated(jar: &CookieJar) -> bool {
    let Some(cookie) = jar.get("auth") else {
        return false;
    };
    todo!()
}

async fn get_download_all(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    if !is_authenticated(&jar) {
        return response_json(
            StatusCode::UNAUTHORIZED,
            json!({"status": "401"}).to_string(),
            &ctx,
        );
    }
    let conn = ctx.conn_lock();
    let posts = Post::list(&conn);
    let posts = if let Ok(posts) = posts {
        posts
    } else {
        let body = json!({
            "status": "500",
            "message": "failed to list posts",
        })
        .to_string();
        return response_json(StatusCode::INTERNAL_SERVER_ERROR, body, &ctx);
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
