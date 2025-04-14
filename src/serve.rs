use crate::ServeArgs;
use crate::data;
use crate::html::page;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::Response;
use axum::http::StatusCode;
use axum::routing::get;
use data::Post;
use rusqlite::Connection;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
struct ServerContext {
    args: ServeArgs,
    conn: Arc<Mutex<Connection>>,
}

fn response(
    status: StatusCode,
    headers: HeaderMap,
    body: &str,
    ctx: &ServerContext,
) -> Response<Body> {
    let mut response: Response<Body> = Response::default();
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    if ctx.args.production {
        response.headers_mut().insert(
            "Strict-Transport-Security",
            HeaderValue::from_static("max-age=604800; preload"), // 1 week.
        );
    }
    *response.body_mut() = Body::from(body.to_string());
    response
}

async fn list_posts(State(ctx): State<ServerContext>) -> Response<Body> {
    let posts = {
        let conn = ctx.conn.lock().unwrap();
        Post::list(&conn).unwrap()
    };
    let posts = posts
        .iter()
        .map(|p| format!("{}: {}", p.created_at, p.content))
        .collect::<Vec<String>>()
        .join("\n");
    let body = page(&posts);
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

pub async fn run(args: &ServeArgs) {
    let conn = data::connect(args).unwrap();
    data::init(&conn);

    let post = Post {
        id: 1,
        created_at: chrono::Utc::now(),
        content: "Hello, World!".to_string(),
    };
    Post::insert(&conn, &post).unwrap();

    let conn = Arc::new(Mutex::new(conn));

    let ctx = ServerContext {
        args: args.clone(),
        conn,
    };
    let app = Router::new().route("/", get(list_posts)).with_state(ctx);

    let addr = format!("0.0.0.0:{}", args.port);
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
