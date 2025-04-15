use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use fedx::ServeArgs;
use fedx::data;
use fedx::serve::ServerContext;
use fedx::serve::app;
use http_body_util::BodyExt;
use rusqlite::Connection;
use std::sync::Arc;
use std::sync::Mutex;
use tower::util::ServiceExt;

trait TestDefault {
    fn test_default() -> Self;
}

impl TestDefault for ServeArgs {
    fn test_default() -> Self {
        Self {
            production: false,
            port: 3000,
            database_path: "/data/db.sqlite".to_string(),
            admin_username: "test-admin".to_string(),
            admin_password: Some("test-password".to_string()),
        }
    }
}

impl TestDefault for Connection {
    fn test_default() -> Self {
        let args = ServeArgs::test_default();
        let conn = data::connect(&args).unwrap();
        data::init(&conn);
        conn
    }
}

#[tokio::test]
async fn test_home() {
    let args = ServeArgs::test_default();
    let conn = Connection::test_default();
    let ctx = ServerContext {
        args: args.clone(),
        conn: Arc::new(Mutex::new(conn)),
    };
    let app = app(ctx);
    let req = Request::builder().uri("/").body(Body::empty()).unwrap();
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap();
    let body: Vec<u8> = body.to_bytes().into();
    let body = String::from_utf8(body).unwrap();
    assert!(body.contains("<ul>"));
}
