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
use std::io::Read;
use tar::Builder;
use tar::Header;
use xz2::read::XzEncoder;

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
    response_json(status, body, ctx)
}

fn unauthorized(ctx: &ServerContext) -> Response<Body> {
    error(ctx, StatusCode::UNAUTHORIZED, "unauthorized")
}

struct SiteData<'a> {
    posts: &'a [Post],
}

fn create_archive(site_data: &SiteData) -> Vec<u8> {
    let mut ar = Builder::new(Vec::new());

    for post in site_data.posts {
        let mut header = Header::new_gnu();
        header.set_size(post.content.len() as u64);
        let path = format!("post/{}.md", post.id);
        header.set_path(&path).unwrap();
        // Without this, the file is not even readable by the user.
        header.set_mode(0o644);
        // Using `+++` for the frontmatter because that is toml in Hugo.
        let content = indoc::formatdoc! {"
            +++
            created = {}
            updated = {}
            +++

            {}
        ", post.created, post.updated, post.content};
        let data = content.as_bytes();
        ar.append_data(&mut header, &path, data).unwrap();
    }

    ar.into_inner().unwrap()
}

fn compress(data: &[u8]) -> Vec<u8> {
    let mut compressor = XzEncoder::new(data, 6);
    let mut compressed = Vec::new();
    compressor.read_to_end(&mut compressed).unwrap();
    compressed
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
    let site_data = SiteData { posts: &posts };
    let data = create_archive(&site_data);
    let body = compress(&data);
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
        .route("/api/download/all.tar.xz", get(get_download_all))
}
