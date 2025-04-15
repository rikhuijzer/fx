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
pub struct ServerContext {
    pub args: ServeArgs,
    pub conn: Arc<Mutex<Connection>>,
}

impl ServerContext {
    pub fn new(args: ServeArgs, conn: Connection) -> Self {
        Self {
            args: args.clone(),
            conn: Arc::new(Mutex::new(conn)),
        }
    }
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
        .map(|p| format!("<li>{}: {}</li>", p.created_at, p.content))
        .collect::<Vec<String>>()
        .join("\n");
    let body = page(&format!("<ul>{}</ul>", posts));
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

async fn style(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = include_str!("static/style.css");
    response(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

pub fn app(ctx: ServerContext) -> Router {
    Router::new()
        .route("/", get(list_posts))
        .route("/static/style.css", get(style))
        .with_state(ctx)
}

pub async fn run(args: &ServeArgs) {
    let conn = data::connect(args).unwrap();
    data::init(&conn);

    let now = chrono::Utc::now();
    Post::insert(&conn, now, "Hello, World!").unwrap();
    Post::insert(&conn, now, "Hello, again!").unwrap();

    let ctx = ServerContext::new(args.clone(), conn);
    let app = app(ctx);
    let addr = format!("0.0.0.0:{}", args.port);
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
