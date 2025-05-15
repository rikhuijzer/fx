use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use fx::ServeArgs;
use fx::data;
use fx::serve::LoginForm;
use fx::serve::ServerContext;
use fx::serve::app;
use http_body_util::BodyExt;
use rusqlite::Connection;
use tower::util::ServiceExt;

pub trait TestDefault {
    fn test_default() -> Self;
}

impl TestDefault for ServeArgs {
    fn test_default() -> Self {
        Self {
            trigger_token: Some("trigger-token".to_string()),
            trigger_owner_repo: Some("test-owner/test-repo".to_string()),
            trigger_branch: "main".to_string(),
            trigger_workflow_id: "ci.yml".to_string(),
            production: false,
            port: 3000,
            database_path: "".to_string(),
            username: "test-admin".to_string(),
            html_lang: "en".to_string(),
            password: Some("test-password".to_string()),
            domain: "".to_string(),
            extra_head: "<meta property='test' content='test'>".to_string(),
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

pub async fn server_context() -> ServerContext {
    let args = ServeArgs::test_default();
    let conn = Connection::test_default();
    let salt = fx_auth::generate_salt();
    ServerContext::new(args, conn, salt).await
}

pub async fn request_body(uri: &str) -> (StatusCode, String) {
    let ctx = server_context().await;
    let app = app(ctx);
    let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
    let response = app.oneshot(req).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap();
    let body: Vec<u8> = body.to_bytes().into();
    let body = String::from_utf8(body).unwrap();
    (status, body)
}

pub async fn request_cookie() -> (ServerContext, String) {
    let args = ServeArgs::test_default();
    let conn = Connection::test_default();
    let salt = fx_auth::generate_salt();
    let ctx = ServerContext::new(args, conn, salt).await;
    let form = LoginForm {
        username: "test-admin".to_string(),
        password: "test-password".to_string(),
    };
    let form_data = serde_urlencoded::to_string(&form).unwrap();
    let req = Request::builder()
        .method("POST")
        .uri("/login")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(form_data))
        .unwrap();
    let response = app(ctx.clone()).oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(response.headers().get("Location").unwrap(), "/");
    let cookie = response.headers().get("Set-Cookie").unwrap();
    let cookie = cookie.to_str().unwrap();
    assert!(cookie.contains("auth="));
    assert!(cookie.contains("Secure"));
    let cookie = cookie.split(";").next().unwrap();
    let auth = cookie.split("=").nth(1).unwrap();
    (ctx, auth.to_string())
}

#[allow(dead_code)]
pub async fn request_body_logged_in(uri: &str) -> (StatusCode, Vec<u8>) {
    let (ctx, auth) = request_cookie().await;
    let req = Request::builder()
        .method("GET")
        .uri(uri)
        .header("Cookie", format!("auth={auth}"))
        .body(Body::empty())
        .unwrap();
    let response = app(ctx.clone()).oneshot(req).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap();
    let body: Vec<u8> = body.to_bytes().into();
    (status, body)
}
