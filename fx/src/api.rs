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
use serde_json::json;
use tar::Builder;
use tar::Header;

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

fn is_authenticated(ctx: &ServerContext, headers: &HeaderMap) -> bool {
    let password = &ctx.args.password;
    let password = if let Some(password) = password {
        password
    } else {
        tracing::warn!("admin password not set");
        return false;
    };
    let header = if let Some(cookie) = headers.get("Authorization") {
        cookie
    } else {
        return false;
    };
    let parts = header
        .to_str()
        .unwrap()
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

async fn get_download_all(State(ctx): State<ServerContext>, headers: HeaderMap) -> Response<Body> {
    if !is_authenticated(&ctx, &headers) {
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

    let mut ar = Builder::new(Vec::new());

    for post in posts {
        let mut header = Header::new_gnu();
        header.set_size(post.content.len() as u64);
        let path = format!("post/{}.md", post.id);
        header.set_path(&path).unwrap();
        // Without this, the file is not even readable by the user.
        header.set_mode(0o644);
        ar.append_data(&mut header, &path, post.content.as_bytes())
            .unwrap();
    }

    let body = ar.into_inner().unwrap();
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
        .route("/api/download/all.tar.gz", get(get_download_all))
}
