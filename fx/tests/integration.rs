mod common;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use common::*;
use fx::ServeArgs;
use fx::serve::LoginForm;
use fx::serve::ServerContext;
use fx::serve::app;
use http_body_util::BodyExt;
use rusqlite::Connection;
use tower::util::ServiceExt;

#[tokio::test]
async fn test_home() {
    let (status, body) = request_body("/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains("Lorem"));
    assert!(body.contains("Dolor"));
    assert!(body.contains("<meta property='test' content='test'>"));
}

#[tokio::test]
async fn test_get_post() {
    let (status, body) = request_body("/posts/1").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Lorem"));

    let (status, body) = request_body("/posts/2").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Dolor"));
}

#[tokio::test]
async fn test_style() {
    let (status, body) = request_body("/static/style.css").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("body {"));
}

#[tokio::test]
async fn test_metadata() {
    let (status, body) = request_body("/posts/1").await;
    assert_eq!(status, StatusCode::OK);
    let lines = body.lines().collect::<Vec<_>>();
    let head_start = lines
        .iter()
        .position(|line| line.contains("<head>"))
        .unwrap();
    let head_end = lines
        .iter()
        .position(|line| line.contains("</head>"))
        .unwrap();
    let head = lines[head_start..head_end + 1].join("\n");
    println!("head:\n{head}");
    assert!(body.contains("<!DOCTYPE html>"));
    assert!(body.contains(
        "<title>Lorem ipsum ut enim ad minim veniam sit amet ipsum lorem con... - site-name</title>"
    ));
    assert!(body.contains("<meta property='og:site_name' content='site-name'/>"));
    assert!(body.contains("<meta property='article:author' content='Test Admin'/>"));
}

#[tokio::test]
async fn test_login_page() {
    let (status, body) = request_body("/login").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("username"));
}

#[tokio::test]
async fn test_login() {
    let args = ServeArgs::test_default();
    let conn = Connection::test_default();
    let salt = fx_auth::generate_salt();
    let ctx = ServerContext::new(args, conn, salt);

    // Valid login.
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
    println!("auth: {auth}");

    // Valid cookie.
    let req = Request::builder()
        .method("GET")
        .uri("/")
        .header("Cookie", format!("auth={auth}"))
        .body(Body::empty())
        .unwrap();
    let response = app(ctx.clone()).oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap();
    let body: Vec<u8> = body.to_bytes().into();
    let body = String::from_utf8(body).unwrap();
    assert!(body.contains("Lorem"));
    assert!(!body.contains("login"));
    assert!(body.contains("logout"));

    // Invalid cookie.
    let req = Request::builder()
        .method("GET")
        .uri("/")
        .header("Cookie", "auth=invalid")
        .body(Body::empty())
        .unwrap();
    let response = app(ctx.clone()).oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap();
    let body: Vec<u8> = body.to_bytes().into();
    let body = String::from_utf8(body).unwrap();
    assert!(body.contains("Lorem"));
    assert!(body.contains("login"));
    assert!(!body.contains("logout"));

    // Wrong password.
    let form = LoginForm {
        username: "test-admin".to_string(),
        password: "wrong-password".to_string(),
    };
    let form_data = serde_urlencoded::to_string(&form).unwrap();
    let req = Request::builder()
        .method("POST")
        .uri("/login")
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(form_data))
        .unwrap();
    let app = app(ctx);
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = response.into_body().collect().await.unwrap();
    let body: Vec<u8> = body.to_bytes().into();
    let body = String::from_utf8(body).unwrap();
    assert!(body.contains("login"));
    assert!(body.contains("Invalid username or password"));
}

#[tokio::test]
async fn test_delete_confirmation() {
    let (status, body) = request_body("/posts/delete/1").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(!body.contains("Are you sure you want to delete this post?"));

    let (status, body) = request_body_logged_in("/posts/delete/1").await;
    assert_eq!(status, StatusCode::OK);
    let body = String::from_utf8(body).unwrap();
    assert!(body.contains("Are you sure you want to delete this post?"));
    assert!(body.contains("Lorem"));
}

#[tokio::test]
async fn test_no_access() {
    let endpoints = ["/backup"];
    for endpoint in endpoints {
        let (status, _body) = request_body(endpoint).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn test_backup() {
    let (status, body) = request_body_logged_in("/backup").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.len() > 0);
}

#[tokio::test]
async fn test_post_add() {
    let (ctx, auth) = request_cookie().await;
    let form = fx::serve::AddPostForm {
        content: "Lorem https://example.com".to_string(),
    };
    let form_data = serde_urlencoded::to_string(&form).unwrap();
    let req = Request::builder()
        .method("POST")
        .uri("/posts/add")
        .header("Cookie", format!("auth={auth}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(form_data))
        .unwrap();
    let response = app(ctx.clone()).oneshot(req).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap();
    let body: Vec<u8> = body.to_bytes().into();
    let body = String::from_utf8(body).unwrap();
    println!("body:\n{body}");
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("example.com"), "redirect to the new post");
    assert!(
        body.contains(r#"<a href="https://example.com">"#),
        "auto autolink"
    );
}

#[tokio::test]
async fn test_get_edit() {
    let (status, body) = request_body_logged_in("/posts/edit/2").await;
    let body = String::from_utf8(body).unwrap();
    assert_eq!(status, StatusCode::OK);
    println!("body:\n{body}");
    assert!(
        body.contains("# Code\n\nDolor sit"),
        "textarea content might be minified"
    );
}

#[tokio::test]
async fn test_post_edit() {
    let (ctx, auth) = request_cookie().await;
    let form = fx::serve::EditPostForm {
        content: "Lorem https://example.com".to_string(),
    };
    let form_data = serde_urlencoded::to_string(&form).unwrap();
    let req = Request::builder()
        .method("POST")
        .uri("/posts/edit/2")
        .header("Cookie", format!("auth={auth}"))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(Body::from(form_data))
        .unwrap();
    let response = app(ctx.clone()).oneshot(req).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap();
    let body: Vec<u8> = body.to_bytes().into();
    let body = String::from_utf8(body).unwrap();
    println!("body:\n{body}");
    assert_eq!(status, StatusCode::OK);
    assert!(!body.contains("# Code"), "text not updated");
    assert!(body.contains("https://example.com"), "text not updated");
}
