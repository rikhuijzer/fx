use crate::ServeArgs;
use crate::data;
use axum::Router;
use axum::routing::get;
use data::Post;

async fn home() -> &'static str {
    "Hello, World!"
}

pub async fn run(args: &ServeArgs) {
    let app = Router::new().route("/", get(home));

    let db = data::init(args).unwrap();
    let post = Post {
        id: 1,
        created_at: chrono::Utc::now(),
        content: "Hello, World!".to_string(),
    };
    Post::insert(&db.conn, &post).unwrap();

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
