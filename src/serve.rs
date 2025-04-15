use crate::ServeArgs;
use crate::data;
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

fn format_post(p: &Post) -> String {
    indoc::formatdoc! {"
        <div class='post'>
            <div class='created_at'>{}</div>
            <a style='text-decoration: none; color: inherit;' href='/p/{}'>
                <div class='content'>{}</div>
            </a>
        </div>
    ", p.created_at, p.id, p.content}
}

async fn list_posts(State(ctx): State<ServerContext>) -> Response<Body> {
    let posts = {
        let conn = ctx.conn.lock().unwrap();
        Post::list(&conn).unwrap()
    };
    let posts = posts
        .iter()
        .map(format_post)
        .collect::<Vec<String>>()
        .join("\n");
    let body = page(&format!("<ul>{}</ul>", posts));
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

async fn style(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = include_str!("static/style.css");
    response(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

async fn show_post(State(ctx): State<ServerContext>, Path(id): Path<i64>) -> Response<Body> {
    let post = Post::get(&ctx.conn.lock().unwrap(), id);
    let post = match post {
        Ok(post) => post,
        Err(_) => return not_found(State(ctx)).await,
    };
    let body = page(&format_post(&post));
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

async fn not_found(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = indoc::indoc! {"
        <div style='text-align: center; margin-top: 100px;'>
            <h1>Not found</h1>
            <p>The page you are looking for does not exist.</p>
        </div>
    "};
    let body = page(body);
    response(StatusCode::NOT_FOUND, HeaderMap::new(), &body, &ctx)
}

pub fn app(ctx: ServerContext) -> Router {
    Router::new()
        .route("/", get(list_posts))
        .route("/static/style.css", get(style))
        // Need to put behind /p/<ID> otherwise /<WRONG LINK> will not be a 404.
        .route("/p/{id}", get(show_post))
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
