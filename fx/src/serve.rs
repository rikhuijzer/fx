use crate::ServeArgs;
use crate::data;
use crate::html::PageSettings;
use crate::html::ToHtml;
use crate::html::page;
use axum::Form;
use axum::Router;
use axum::body::Body;
use axum::extract::Path;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::Response;
use axum::http::StatusCode;
use axum::response::Redirect;
use axum::routing::get;
use axum::routing::post;
use axum_extra::extract::CookieJar;
use data::Post;
use fx_auth::Login;
use fx_auth::Salt;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
pub struct ServerContext {
    pub args: ServeArgs,
    pub conn: Arc<Mutex<Connection>>,
    pub salt: Salt,
}

impl ServerContext {
    pub fn new(args: ServeArgs, conn: Connection, salt: Salt) -> Self {
        Self {
            args: args.clone(),
            conn: Arc::new(Mutex::new(conn)),
            salt,
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

fn is_logged_in(ctx: &ServerContext, jar: &CookieJar) -> bool {
    let password = match &ctx.args.admin_password {
        Some(password) => password,
        None => {
            tracing::warn!("admin password not set");
            return false;
        }
    };
    let login = Login {
        username: Some(ctx.args.admin_username.clone()),
        password: Some(password.clone()),
    };
    fx_auth::is_logged_in(&ctx.salt, &login, jar)
}

async fn list_posts(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let posts = {
        let conn = ctx.conn.lock().unwrap();
        Post::list(&conn).unwrap()
    };
    let posts = posts
        .iter()
        .map(|p| p.to_html())
        .collect::<Vec<String>>()
        .join("\n");
    let settings = PageSettings::new("", is_logged_in, true);
    let body = page(&ctx, &settings, &posts);
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
    let settings = PageSettings::new(&title, false, false);
    let body = page(&ctx, &settings, &post.to_html());
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

async fn not_found(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = indoc::indoc! {"
        <div style='text-align: center; margin-top: 100px;'>
            <h1>Not found</h1>
            <p>The page you are looking for does not exist.</p>
        </div>
    "};
    let settings = PageSettings::new("not found", false, false);
    let body = page(&ctx, &settings, body);
    response(StatusCode::NOT_FOUND, HeaderMap::new(), &body, &ctx)
}

async fn get_login(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::html::login(&ctx, None);
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

async fn post_login(
    State(ctx): State<ServerContext>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Result<(CookieJar, Redirect), Response<Body>> {
    let password = match &ctx.args.admin_password {
        Some(password) => password,
        None => {
            tracing::warn!("admin password not set");
            return Err(response(
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                "Admin password not set",
                &ctx,
            ));
        }
    };
    let actual = Login {
        username: Some(ctx.args.admin_username.clone()),
        password: Some(password.clone()),
    };
    let received = Login {
        username: Some(form.username),
        password: Some(form.password),
    };
    let new_jar = fx_auth::handle_login(&ctx.salt, &actual, &received, jar.clone());
    match new_jar {
        Some(jar) => Ok((jar, Redirect::to("/"))),
        None => {
            let body = crate::html::login(&ctx, Some("Invalid username or password"));
            Err(response(
                StatusCode::UNAUTHORIZED,
                HeaderMap::new(),
                &body,
                &ctx,
            ))
        }
    }
}

async fn get_logout(
    State(_secrets): State<ServerContext>,
    jar: CookieJar,
) -> (CookieJar, Redirect) {
    let updated_jar = fx_auth::handle_logout(jar.clone());
    (updated_jar, Redirect::to("/"))
}

pub fn app(ctx: ServerContext) -> Router {
    Router::new()
        .route("/", get(list_posts))
        .route("/login", get(get_login))
        .route("/login", post(post_login))
        .route("/logout", get(get_logout))
        // Need to put behind /p/<ID> otherwise /<WRONG LINK> will not be a 404.
        .route("/p/{id}", get(show_post))
        .route("/static/style.css", get(style))
        .fallback(not_found)
        .with_state(ctx)
}

/// Return the salt by either generating a new one or reading it from the db.
///
/// Re-using the salt between sessions allows users to keep logged in even when
/// the server restarts.
fn obtain_salt(conn: &Connection) -> Salt {
    let salt = data::Kv::get(conn, "salt");
    match salt {
        Ok(salt) => salt.value.try_into().unwrap(),
        Err(_) => {
            let salt = fx_auth::generate_salt();
            data::Kv::insert(conn, "salt", &salt).unwrap();
            salt
        }
    }
}

pub async fn run(args: &ServeArgs) {
    let conn = data::connect(args).unwrap();
    data::init(args, &conn);
    let salt = obtain_salt(&conn);
    let ctx = ServerContext::new(args.clone(), conn, salt);
    let app = app(ctx);
    let addr = format!("0.0.0.0:{}", args.port);
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
