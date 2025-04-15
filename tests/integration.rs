use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use fx::ServeArgs;
use fx::data;
use fx::serve::ServerContext;
use fx::serve::app;
use http_body_util::BodyExt;
use rusqlite::Connection;
use tower::util::ServiceExt;

trait TestDefault {
    fn test_default() -> Self;
}

impl TestDefault for ServeArgs {
    fn test_default() -> Self {
        Self {
            production: false,
            port: 3000,
            database_path: "".to_string(),
            admin_username: "test-admin".to_string(),
            admin_name: "Test Admin".to_string(),
            admin_password: Some("test-password".to_string()),
        }
    }
}

impl TestDefault for Connection {
    fn test_default() -> Self {
        let args = ServeArgs::test_default();
        let conn = data::connect(&args).unwrap();
        data::init(&args, &conn);
        conn
    }
}

async fn request_body(uri: &str) -> String {
    let args = ServeArgs::test_default();
    let conn = Connection::test_default();
    let ctx = ServerContext::new(args, conn);
    let app = app(ctx);
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap();
    let body: Vec<u8> = body.to_bytes().into();
    String::from_utf8(body).unwrap()
}

#[tokio::test]
async fn test_home() {
    let body = request_body("/").await;
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("lorem ipsum"));
    assert!(body.contains("dolor sit amet"));
}

#[tokio::test]
async fn test_post() {
    let body = request_body("/p/1").await;
    assert!(body.contains("lorem ipsum"));

    let body = request_body("/p/2").await;
    assert!(body.contains("dolor sit amet"));
}

#[tokio::test]
async fn test_style() {
    let body = request_body("/static/style.css").await;
    assert!(body.contains("body {"));
}
