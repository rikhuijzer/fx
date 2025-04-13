use crate::data;
use data::Post;
use axum::Router;
use axum::routing::get;

async fn home() -> &'static str {
    "Hello, World!"
}

pub async fn run(production: bool) {
    let app = Router::new().route("/", get(home));

    let db = data::init(production).unwrap();
    let post = Post {
        id: 1,
        created_at: chrono::Utc::now(),
        content: "Hello, World!".to_string(),
    };
    Post::insert(&db.conn, &post).unwrap();

    let port = 3000;
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
