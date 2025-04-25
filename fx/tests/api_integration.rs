mod common;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use chrono::Utc;
use common::*;
use fx::serve::app;
use http_body_util::BodyExt;
use std::io::Cursor;
use std::io::Read;
use tar::Archive;
use tar::Entry;
use tower::util::ServiceExt;
use xz2::read::XzDecoder;

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
    let uri = "/api/download/all.tar.xz";
    let (status, body) = request_body_authenticated(uri).await;
    assert_eq!(status, StatusCode::OK);

    let body = Cursor::new(body);
    let decompressed = XzDecoder::new(body);
    let mut ar = Archive::new(decompressed);
    // Do not collect the entries because it moves the stream pointer.
    let mut entries = ar.entries().unwrap();
    fn path<T: std::io::Read>(entry: &Entry<T>) -> String {
        entry.path().unwrap().to_str().unwrap().to_string()
    }
    let mut first = entries.next().unwrap().unwrap();
    // SQLite is 1-indexed.
    assert!(path(&first).contains("post/1.md"));

    let mut content = String::new();
    first.read_to_string(&mut content).unwrap();
    println!("content:\n{content}");
    let lines = content.lines().collect::<Vec<_>>();
    assert_eq!(lines[0], "---");
    let today = Utc::now().format("%Y-%m-%d").to_string();
    assert!(lines[1].contains(&format!("created: '{today}")));
    assert!(lines[2].contains(&format!("updated: '{today}")));
    assert_eq!(lines[3], "---");
    assert!(lines[4].is_empty());
    assert!(lines[5].contains("Lorem"));

    let second = entries.next().unwrap().unwrap();
    assert!(path(&second).contains("post/2.md"));
    assert!(entries.next().is_none());
}
