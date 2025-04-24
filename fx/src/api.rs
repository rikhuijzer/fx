use axum::Router;
use axum::extract::State;
use axum::routing::get;
use crate::serve::ServerContext;
use serde_json::json;
use crate::serve::response;
use axum::http::StatusCode;
use axum::http::header::HeaderMap;
use axum::http::header::HeaderValue;
use axum::http::Response;
use axum::body::Body;

async fn get_api(State(ctx): State<ServerContext>) -> Response<Body> {
    let domain = &ctx.args.domain;
    let domain = if let Some(domain) = domain {
        domain
    } else {
        ""
    };
    let domain = if domain == "localhost" {
        "http://localhost:3000"
    } else {
        &format!("https://{domain}")
    };
    let json = json!({
        "download_all_url": format!("{domain}/download/all"),
    }).to_string();
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/json"),
    );
    response(StatusCode::OK, headers, json, &ctx)
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router.clone().route("/api", get(get_api))
}
