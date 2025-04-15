use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use fedx::ServeArgs;
use fedx::data;
use fedx::serve::ServerContext;
use fedx::serve::app;
use http_body_util::BodyExt;
use std::sync::Arc;
use std::sync::Mutex;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_home() {
    let args = ServeArgs {
        production: false,
        port: 3000,
        database_path: "/data/db.sqlite".to_string(),
        admin_username: "admin".to_string(),
        admin_password: None,
    };
    let conn = data::connect(&args).unwrap();
    data::init(&conn);
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
