use crate::ServeArgs;
use crate::data;
use crate::html::HtmlCtx;
use crate::html::PageSettings;
use crate::html::ToHtml;
use crate::html::Top;
use crate::html::page;
use axum::Form;
use axum::Router;
use axum::body::Body;
use axum::extract::Path;
use axum::extract::Request;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::HeaderValue;
use axum::http::Response;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::Redirect;
use axum::routing::get;
use axum::routing::post;
use axum_extra::extract::CookieJar;
use chrono::Utc;
use data::Post;
use fx_auth::Login;
use fx_auth::Salt;
use http_body_util::BodyExt;
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

fn list_posts(ctx: &ServerContext, is_logged_in: bool) -> String {
    let posts = {
        let conn = ctx.conn.lock().unwrap();
        Post::list(&conn).unwrap()
    };
    let hctx = HtmlCtx::new(is_logged_in, true);
    posts
        .iter()
        .map(|p| {
            format!(
                "<a class='unstyled-link' href='/post/{}'>{}</a>",
                p.id,
                p.to_html(&hctx)
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

async fn get_posts(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let settings = PageSettings::new("", is_logged_in, true, Top::Homepage);
    let posts = list_posts(&ctx, is_logged_in);
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

async fn get_delete(
    State(ctx): State<ServerContext>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return not_found(State(ctx.clone())).await;
    }
    let post = Post::get(&ctx.conn.lock().unwrap(), id);
    let post = match post {
        Ok(post) => post,
        Err(_) => return not_found(State(ctx.clone())).await,
    };
    let settings = PageSettings::new(&post.content, false, false, Top::GoHome);
    let hctx = HtmlCtx::new(false, false);
    let delete_button = indoc::formatdoc! {r#"
        <div class='center medium-text' style='font-weight: bold;'>
            <p>Are you sure you want to delete this post? This action cannot be undone.</p>
            <form action='/post/delete/{id}' method='post'>
                <button type='submit'>delete</button>
            </form>
            <br>
        </div>
    "#};
    let body = format!("{}\n{}", delete_button, post.to_html(&hctx));
    let body = page(&ctx, &settings, &body);
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

async fn get_post(
    State(ctx): State<ServerContext>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let post = Post::get(&ctx.conn.lock().unwrap(), id);
    let post = match post {
        Ok(post) => post,
        Err(_) => return not_found(State(ctx)).await,
    };
    let title = truncate(&post.content);
    let settings = PageSettings::new(&title, false, false, Top::GoHome);
    let hctx = HtmlCtx::new(false, false);
    let mut body = post.to_html(&hctx);
    if is_logged_in {
        body = format!("{}\n{body}", crate::html::edit_post_buttons(&ctx, &post));
    }
    let body = page(&ctx, &settings, &body);
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

async fn not_found(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = indoc::indoc! {"
        <div style='text-align: center; margin-top: 100px;'>
            <h1>Not found</h1>
            <p>The page you are looking for does not exist.</p>
        </div>
    "};
    let settings = PageSettings::new("not found", false, false, Top::GoHome);
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

async fn get_logout(State(_ctx): State<ServerContext>, jar: CookieJar) -> (CookieJar, Redirect) {
    let updated_jar = fx_auth::handle_logout(jar.clone());
    (updated_jar, Redirect::to("/"))
}

async fn post_delete(
    State(ctx): State<ServerContext>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Result<Redirect, Response<Body>> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return Err(response(
            StatusCode::UNAUTHORIZED,
            HeaderMap::new(),
            "Unauthorized",
            &ctx,
        ));
    }
    let conn = ctx.conn.lock().unwrap();
    Post::delete(&conn, id).unwrap();
    Ok(Redirect::to("/"))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostForm {
    pub content: String,
}

async fn post_add(
    State(ctx): State<ServerContext>,
    jar: CookieJar,
    req: Request,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let settings = PageSettings::new("", is_logged_in, false, Top::GoBack);
    let (_, body) = req.into_parts();
    let bytes = body
        .collect()
        .await
        .map_err(|_err| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to read request body",
            )
                .into_response()
        })
        .unwrap()
        .to_bytes();
    let bytes = bytes.to_vec();
    let content = String::from_utf8(bytes).unwrap();
    let preview = content.contains("preview");
    let content = content[8..].to_string();
    let content = content.split("&").next().unwrap();
    if preview {
        let preview = crate::html::post_preview(content);
        let body = page(&ctx, &settings, &preview);
        response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
    } else {
        let now = Utc::now();
        let conn = ctx.conn.lock().unwrap();
        let post = Post::insert(&conn, now, content);
        if post.is_err() {
            return response(
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                "Failed to insert post",
                &ctx,
            );
        };
        let mut headers = HeaderMap::new();
        headers.insert("Location", HeaderValue::from_str("/").unwrap());
        response(StatusCode::SEE_OTHER, headers, "", &ctx)
    }
}

pub fn app(ctx: ServerContext) -> Router {
    Router::new()
        .route("/", get(get_posts))
        .route("/post/delete/{id}", get(get_delete))
        .route("/post/delete/{id}", post(post_delete))
        .route("/post/add", post(post_add))
        .route("/login", get(get_login))
        .route("/login", post(post_login))
        .route("/logout", get(get_logout))
        // Need to put behind /post/<ID> otherwise /<WRONG LINK> will not be a 404.
        .route("/post/{id}", get(get_post))
        .route("/static/style.css", get(style))
        .fallback(not_found)
        .with_state(ctx)
}

/// Return the salt by either generating a new one or reading it from the db.
///
/// Re-using the salt between sessions allows users to keep logged in even when
/// the server restarts.
fn obtain_salt(args: &ServeArgs, conn: &Connection) -> Salt {
    if args.production {
        let salt = data::Kv::get(conn, "salt");
        match salt {
            Ok(salt) => salt.value.try_into().unwrap(),
            Err(_) => {
                let salt = fx_auth::generate_salt();
                data::Kv::insert(conn, "salt", &salt).unwrap();
                salt
            }
        }
    } else {
        // Allow the login to persist across restarts.
        b"nblVMlxYtvt0rxo3BML3zw".to_owned()
    }
}

pub async fn run(args: &ServeArgs) {
    let conn = data::connect(args).unwrap();
    data::init(args, &conn);
    let salt = obtain_salt(args, &conn);
    let ctx = ServerContext::new(args.clone(), conn, salt);
    let app = app(ctx);
    let addr = format!("0.0.0.0:{}", args.port);
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
