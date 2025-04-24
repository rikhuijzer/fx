mod common;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use common::*;
use fx::serve::app;
use http_body_util::BodyExt;
use std::fs::File;
use std::io::Write;
use tar::Archive;
use tempfile::tempdir;
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
    assert!(serde_json::from_slice::<serde_json::Value>(&body).is_err());
    assert_eq!(status, StatusCode::OK);

    let temp_dir = tempdir().unwrap();

    let archive_path = temp_dir.path().join("archive.tar.gz");
    let mut file = File::create(archive_path).unwrap();
    file.write_all(&body).unwrap();
    let mut ar = Archive::new(file);
    for entry in ar.entries().unwrap() {
        let entry = entry.unwrap();
        let path = entry.path().unwrap();
        println!("{}", path.display());
    }
    assert!(false);
}
