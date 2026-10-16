#[allow(dead_code)]
mod common;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use common::server_context;
use fx::serve::app;
use http_body_util::BodyExt;
use tower::util::ServiceExt;

// Use valid payloads and existing resources so extractor errors or missing
// records cannot accidentally make an unprotected endpoint pass these tests.
async fn requires_auth(
    method: &str,
    uri: &str,
    content_type: &str,
    body: &str,
    expected_status: StatusCode,
) -> Vec<String> {
    let mut ctx = server_context().await;
    ctx.args.trigger_token = None;
    let router = app(ctx);
    let mut response_bodies = Vec::new();
    for invalid_credentials in [false, true] {
        let mut req = Request::builder()
            .method(method)
            .uri(uri)
            .header("Content-Type", content_type);
        if invalid_credentials {
            req = if uri.starts_with("/api/") {
                req.header("Authorization", "Bearer wrong-password")
            } else {
                req.header("Cookie", "auth=invalid")
            };
        }
        let req = req.body(Body::from(body.to_owned())).unwrap();
        let response = router.clone().oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            expected_status,
            "{method} {uri} (invalid credentials: {invalid_credentials})",
        );
        let body = response.into_body().collect().await.unwrap().to_bytes();
        response_bodies.push(String::from_utf8(body.to_vec()).unwrap());
    }
    response_bodies
}

// These post routes currently deny access with a 404.
#[tokio::test]
async fn get_post_delete() {
    let bodies = requires_auth(
        "GET",
        "/posts/delete/1",
        "text/plain",
        "",
        StatusCode::NOT_FOUND,
    )
    .await;
    for body in bodies {
        assert!(!body.contains("Are you sure you want to delete this post?"));
    }
}

#[tokio::test]
async fn post_post_add() {
    requires_auth(
        "POST",
        "/posts/add",
        "application/x-www-form-urlencoded",
        "content=Unauthorized&publish=Publish",
        StatusCode::NOT_FOUND,
    )
    .await;
}

#[tokio::test]
async fn post_post_edit() {
    requires_auth(
        "POST",
        "/posts/edit/1",
        "application/x-www-form-urlencoded",
        "content=Unauthorized&publish=Publish",
        StatusCode::NOT_FOUND,
    )
    .await;
}

#[tokio::test]
async fn post_post_delete() {
    requires_auth(
        "POST",
        "/posts/delete/1",
        "text/plain",
        "",
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn get_files() {
    requires_auth("GET", "/files", "text/plain", "", StatusCode::UNAUTHORIZED).await;
}

#[tokio::test]
async fn post_file_add() {
    let body = "--boundary\r\nContent-Disposition: form-data; name=\"file\"; filename=\"unauthorized.txt\"\r\nContent-Type: text/plain\r\n\r\nUnauthorized\r\n--boundary--\r\n";
    requires_auth(
        "POST",
        "/files/add",
        "multipart/form-data; boundary=boundary",
        body,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn get_file_delete() {
    requires_auth(
        "GET",
        "/files/delete/69b83ddf8f65695f",
        "text/plain",
        "",
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn post_file_delete() {
    requires_auth(
        "POST",
        "/files/delete/69b83ddf8f65695f",
        "text/plain",
        "",
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn get_file_rename() {
    requires_auth(
        "GET",
        "/files/rename/69b83ddf8f65695f",
        "text/plain",
        "",
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn post_file_rename() {
    requires_auth(
        "POST",
        "/files/rename/69b83ddf8f65695f",
        "application/x-www-form-urlencoded",
        "filename=unauthorized.txt",
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn get_settings() {
    requires_auth(
        "GET",
        "/settings",
        "text/plain",
        "",
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn post_settings() {
    let body = "site_name=Unauthorized&site_description=Unauthorized&author_name=Unauthorized&about=Unauthorized&extra_head=";
    requires_auth(
        "POST",
        "/settings",
        "application/x-www-form-urlencoded",
        body,
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn get_api_download() {
    requires_auth(
        "GET",
        "/api/download/all.tar.xz",
        "text/plain",
        "",
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[tokio::test]
async fn put_api_about() {
    requires_auth(
        "PUT",
        "/api/settings/about",
        "text/plain",
        "Unauthorized",
        StatusCode::UNAUTHORIZED,
    )
    .await;
}
