use crate::ServeArgs;
use crate::data;
use crate::html::ToHtml;
use crate::html::page;
use axum::Router;
use axum::body::Body;
use axum::extract::Path;
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
        .map(|p| p.to_html())
        .collect::<Vec<String>>()
        .join("\n");
    let body = page(&ctx, "", true, &posts);
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

async fn style(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = include_str!("static/style.css");
    response(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

fn truncate(text: &str) -> String {
    let max_length = 40;
    let mut text = text.to_string();
    if text.len() > max_length {
        let mut pos = max_length;
        while pos > 0 && !text.is_char_boundary(pos) {
            pos -= 1;
        }
        text.truncate(pos);
        text.push_str("...");
    }
    text.to_string()
}

async fn show_post(State(ctx): State<ServerContext>, Path(id): Path<i64>) -> Response<Body> {
    let post = Post::get(&ctx.conn.lock().unwrap(), id);
    let post = match post {
        Ok(post) => post,
        Err(_) => return not_found(State(ctx)).await,
    };
    let title = truncate(&post.content);
    let body = page(&ctx, &title, false, &post.to_html());
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

async fn not_found(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = indoc::indoc! {"
        <div style='text-align: center; margin-top: 100px;'>
            <h1>Not found</h1>
            <p>The page you are looking for does not exist.</p>
        </div>
    "};
    let body = page(&ctx, "not found", false, body);
    response(StatusCode::NOT_FOUND, HeaderMap::new(), &body, &ctx)
}

async fn login(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::html::login(&ctx);
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

pub fn app(ctx: ServerContext) -> Router {
    Router::new()
        .route("/", get(list_posts))
        .route("/login", get(login))
        // Need to put behind /p/<ID> otherwise /<WRONG LINK> will not be a 404.
        .route("/p/{id}", get(show_post))
        .route("/static/style.css", get(style))
        .fallback(not_found)
        .with_state(ctx)
}

pub async fn run(args: &ServeArgs) {
    let conn = data::connect(args).unwrap();
    data::init(args, &conn);

    let ctx = ServerContext::new(args.clone(), conn);
    let app = app(ctx);
    let addr = format!("0.0.0.0:{}", args.port);
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
