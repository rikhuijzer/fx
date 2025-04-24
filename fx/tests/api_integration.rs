mod common;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use common::*;
use fx::serve::app;
use http_body_util::BodyExt;
use std::io::Cursor;
use tar::Archive;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_api() {
    let (status, _body) = request_body("/api").await;
    assert_eq!(status, StatusCode::OK);
}

#[allow(dead_code)]
pub async fn request_body_authenticated(uri: &str) -> (StatusCode, Vec<u8>) {
    let ctx = server_context();
    let auth = ctx.args.password.clone().unwrap();
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header("Authorization", format!("Bearer {auth}"))
        .body(Body::empty())
        .unwrap();
    let response = app(ctx.clone()).oneshot(req).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap();
    let body: Vec<u8> = body.to_bytes().into();
    (status, body)
}

#[tokio::test]
async fn test_download_all() {
    let (status, body) = request_body_authenticated("/api/download/all.tar.gz").await;
    assert_eq!(status, StatusCode::OK);

    let cursor = Cursor::new(body);
    let mut ar = Archive::new(cursor);
    let entries = ar.entries().unwrap();
    let entries = entries.collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    // SQLite is 1-indexed.
    assert!(entries[0].as_ref().unwrap().path().unwrap().to_str().unwrap().contains("post/1.md"));
    assert!(entries[1].as_ref().unwrap().path().unwrap().to_str().unwrap().contains("post/2.md"));
}
