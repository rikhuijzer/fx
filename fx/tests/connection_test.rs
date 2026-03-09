mod common;

use axum::body::Body;
use axum::extract::Request;
use axum::http::StatusCode;
use common::*;
use fx::serve::app;
use std::time::Duration;
use tower::util::ServiceExt;

/// Verify that many concurrent requests all complete without deadlocking.
#[tokio::test]
async fn test_concurrent_requests_no_deadlock() {
    let ctx = server_context().await;
    let n = 50;
    let mut handles = Vec::with_capacity(n);

    for _ in 0..n {
        let ctx = ctx.clone();
        let handle = tokio::spawn(async move {
            let app = app(ctx);
            let req = Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap();
            let response = app.oneshot(req).await.unwrap();
            response.status()
        });
        handles.push(handle);
    }

    // All requests must complete within 10 seconds.
    let mut ok_count = 0;
    for handle in handles {
        let status = tokio::time::timeout(Duration::from_secs(10), handle)
            .await
            .expect("request timed out — possible deadlock")
            .expect("task panicked");
        assert_eq!(status, StatusCode::OK);
        ok_count += 1;
    }
    assert_eq!(ok_count, n);
}

/// Verify the server remains responsive after a burst of concurrent requests.
#[tokio::test]
async fn test_responsive_after_burst() {
    let ctx = server_context().await;

    // Send a burst of 30 concurrent requests.
    let mut handles = Vec::new();
    for _ in 0..30 {
        let ctx = ctx.clone();
        handles.push(tokio::spawn(async move {
            let app = app(ctx);
            let req = Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap();
            app.oneshot(req).await.unwrap().status()
        }));
    }
    for handle in handles {
        let status = handle.await.expect("task panicked");
        assert_eq!(status, StatusCode::OK);
    }

    // After the burst, a single request must still work immediately.
    let app = app(ctx);
    let req = Request::builder()
        .uri("/")
        .body(Body::empty())
        .unwrap();
    let response = tokio::time::timeout(Duration::from_secs(5), app.oneshot(req))
        .await
        .expect("post-burst request timed out — server unresponsive")
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

/// Verify that the blogroll page is accessible even under concurrent load.
#[tokio::test]
async fn test_blogroll_accessible_under_load() {
    let ctx = server_context().await;

    // Send concurrent requests to both / and /blogroll.
    let mut handles = Vec::new();
    for i in 0..20 {
        let ctx = ctx.clone();
        let uri: &'static str = if i % 2 == 0 { "/" } else { "/blogroll" };
        handles.push(tokio::spawn(async move {
            let app = app(ctx);
            let req = Request::builder()
                .uri(uri)
                .body(Body::empty())
                .unwrap();
            let resp = app.oneshot(req).await.unwrap();
            (uri, resp.status())
        }));
    }

    for handle in handles {
        let (uri, status) = tokio::time::timeout(Duration::from_secs(10), handle)
            .await
            .expect("blogroll request timed out")
            .expect("task panicked");
        assert_eq!(status, StatusCode::OK, "failed for {uri}");
    }
}

/// Start a real TCP server and verify basic HTTP responsiveness.
///
/// Note: uses `axum::serve` rather than the production `serve_with_timeouts`,
/// so this does not exercise the idle connection timeout or TCP keepalive.
#[tokio::test]
async fn test_real_tcp_server_responds() {
    let ctx = server_context().await;
    let app = app(ctx);

    // Bind to a random available port.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    // Spawn the server.
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    // Give the server a moment to start accepting.
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Send a real HTTP request over TCP.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .unwrap();
    let url = format!("http://{addr}/");
    let resp = client.get(&url).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert!(body.contains("<!DOCTYPE html>"));
}

/// Start a real TCP server, send many concurrent connections, then verify
/// the server is still responsive.
///
/// Note: uses `axum::serve` rather than the production `serve_with_timeouts`,
/// so this does not exercise the idle connection timeout or TCP keepalive.
#[tokio::test]
async fn test_real_tcp_concurrent_connections() {
    let ctx = server_context().await;
    let app = app(ctx);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();

    let url = format!("http://{addr}/");

    // Fire 50 concurrent requests.
    let mut handles = Vec::new();
    for _ in 0..50 {
        let client = client.clone();
        let url = url.clone();
        handles.push(tokio::spawn(async move {
            client.get(&url).send().await.unwrap().status().as_u16()
        }));
    }

    for handle in handles {
        let status = tokio::time::timeout(Duration::from_secs(15), handle)
            .await
            .expect("concurrent TCP request timed out — possible deadlock")
            .expect("task panicked");
        assert_eq!(status, 200);
    }

    // Verify the server is still alive after the burst.
    let resp = tokio::time::timeout(
        Duration::from_secs(5),
        client.get(&url).send(),
    )
    .await
    .expect("post-burst TCP request timed out")
    .unwrap();
    assert_eq!(resp.status(), 200);
}
