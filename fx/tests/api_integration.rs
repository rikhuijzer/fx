mod common;

use axum::http::StatusCode;
use common::*;

#[tokio::test]
async fn test_api() {
    let (status, _body) = request_body("/api").await;
    assert_eq!(status, StatusCode::OK);
}
