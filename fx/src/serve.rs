use crate::ServeArgs;
use crate::data;
use crate::data::SqliteDateTime;
use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use crate::html::post_to_html;
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

fn response<D: Sized + ToString>(
    status: StatusCode,
    headers: HeaderMap,
    body: D,
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
    let password = match &ctx.args.password {
        Some(password) => password,
        None => {
            tracing::warn!("admin password not set");
            return false;
        }
    };
    let login = Login {
        username: Some(ctx.args.username.clone()),
        password: Some(password.clone()),
    };
    fx_auth::is_logged_in(&ctx.salt, &login, jar)
}

fn list_posts(ctx: &ServerContext, _is_logged_in: bool) -> String {
    let mut posts = {
        let conn = ctx.conn.lock().unwrap();
        Post::list(&conn).unwrap()
    };
    posts
        .iter_mut()
        .map(|p| {
            crate::md::sanitize_preview(p);
            format!(
                "<a class='unstyled-link' href='/post/{}'>{}</a>",
                p.id,
                post_to_html(p, true)
            )
        })
        .collect::<Vec<String>>()
        .join("\n")
}

async fn get_posts(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let extra_head = &ctx.args.extra_head;
    let settings = PageSettings::new("", is_logged_in, true, Top::Homepage, extra_head);
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
    let extra_head = &ctx.args.extra_head;
    let settings = PageSettings::new(&post.content, false, false, Top::GoHome, extra_head);
    let delete_button = indoc::formatdoc! {r#"
        <div class='center medium-text' style='font-weight: bold;'>
            <p>Are you sure you want to delete this post? This action cannot be undone.</p>
            <form action='/post/delete/{id}' method='post'>
                <button type='submit'>delete</button>
            </form>
            <br>
        </div>
    "#};
    let body = format!("{}\n{}", delete_button, post_to_html(&post, false));
    let body = page(&ctx, &settings, &body);
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

async fn get_edit(
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
    let body = crate::html::edit_post_form(&post);
    let settings = PageSettings::new(
        &post.content,
        is_logged_in,
        false,
        Top::GoBack,
        &ctx.args.extra_head,
    );
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
    let author = &ctx.args.full_name;
    let created = &post.created;
    let updated = &post.updated;
    let extra_head = indoc::formatdoc! {r#"
        <meta property="article:author" content="{author}"/>
        <meta property="article:published_time" content="{created}"/>
        <meta property="article:modified_time" content="{updated}"/>
        {}
    "#, ctx.args.extra_head};
    let settings = PageSettings::new(&title, is_logged_in, false, Top::GoHome, &extra_head);
    let mut body = post_to_html(&post, false);
    if is_logged_in {
        body = format!("{}\n{body}", crate::html::edit_post_buttons(&ctx, &post));
    }
    let body = page(&ctx, &settings, &body);
    response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
}

async fn not_found(State(ctx): State<ServerContext>) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &CookieJar::new());
    let body = indoc::indoc! {"
        <div style='text-align: center; margin-top: 100px;'>
            <h1>Not found</h1>
            <p>The page you are looking for does not exist.</p>
        </div>
    "};
    let extra_head = &ctx.args.extra_head;
    let settings = PageSettings::new("not found", is_logged_in, false, Top::GoHome, extra_head);
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
    let password = match &ctx.args.password {
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
        username: Some(ctx.args.username.clone()),
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
pub struct EditPostForm {
    pub created: String,
    pub updated: String,
    pub content: String,
}

async fn post_edit(
    State(ctx): State<ServerContext>,
    jar: CookieJar,
    Path(id): Path<i64>,
    req: Request,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return not_found(State(ctx)).await;
    }
    let extra_head = &ctx.args.extra_head;
    let settings = PageSettings::new("", is_logged_in, false, Top::GoBack, extra_head);
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
    let publish = content.contains("publish=Publish");
    let form = serde_urlencoded::from_str::<EditPostForm>(&content).unwrap();
    if publish {
        let conn = ctx.conn.lock().unwrap();
        let post = Post {
            id,
            created: SqliteDateTime::from_sqlite(&form.created),
            updated: SqliteDateTime::from_sqlite(&form.updated),
            content: form.content,
        };
        let post = post.update(&conn);
        if post.is_err() {
            return response(
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                format!("Failed to update post: {}", post.err().unwrap()),
                &ctx,
            );
        };
        let mut headers = HeaderMap::new();
        headers.insert("Location", HeaderValue::from_str("/").unwrap());
        response(StatusCode::SEE_OTHER, headers, "", &ctx)
    } else {
        let post = Post {
            id: 0,
            created: SqliteDateTime::from_sqlite(&form.created),
            updated: SqliteDateTime::from_sqlite(&form.updated),
            content: form.content,
        };
        let preview = crate::html::post_to_html(&post, false);
        let body = page(&ctx, &settings, &preview);
        response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AddPostForm {
    pub content: String,
}

async fn post_add(
    State(ctx): State<ServerContext>,
    jar: CookieJar,
    req: Request,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return not_found(State(ctx)).await;
    }
    let extra_head = &ctx.args.extra_head;
    let settings = PageSettings::new("", is_logged_in, false, Top::GoBack, extra_head);
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
    let publish = content.contains("publish=Publish");
    let form = serde_urlencoded::from_str::<AddPostForm>(&content).unwrap();
    if publish {
        let now = Utc::now();
        let conn = ctx.conn.lock().unwrap();
        let post = Post::insert(&conn, now, now, &form.content);
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
    } else {
        let post = Post {
            id: 0,
            created: Utc::now(),
            updated: Utc::now(),
            content: form.content,
        };
        let preview = crate::html::post_to_html(&post, false);
        let body = page(&ctx, &settings, &preview);
        response(StatusCode::OK, HeaderMap::new(), &body, &ctx)
    }
}

async fn get_webfinger(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::ap::webfinger(&ctx);
    let body = match body {
        Some(body) => body,
        None => return not_found(State(ctx)).await,
    };
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/jrd+json; charset=utf-8"),
    );
    response(StatusCode::OK, headers, &body, &ctx)
}

pub fn app(ctx: ServerContext) -> Router {
    Router::new()
        .route("/", get(get_posts))
        .route("/post/delete/{id}", get(get_delete))
        .route("/post/delete/{id}", post(post_delete))
        .route("/post/edit/{id}", get(get_edit))
        .route("/post/edit/{id}", post(post_edit))
        .route("/post/add", post(post_add))
        .route("/login", get(get_login))
        .route("/login", post(post_login))
        .route("/logout", get(get_logout))
        // Need to put behind /post/<ID> otherwise /<WRONG LINK> will not be a 404.
        .route("/post/{id}", get(get_post))
        .route("/static/style.css", get(style))
        .route("/.well-known/webfinger", get(get_webfinger))
        .fallback(not_found)
        .with_state(ctx)
}

/// Return the salt by either generating a new one or reading it from the db.
///
/// Re-using the salt between sessions allows users to keep logged in even when
/// the server restarts.
pub fn obtain_salt(args: &ServeArgs, conn: &Connection) -> Salt {
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
