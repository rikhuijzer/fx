use crate::ServeArgs;
use crate::data;
use crate::data::Post;
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
use fx_auth::Login;
use fx_auth::Salt;
use http_body_util::BodyExt;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;

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
    pub fn conn_lock(&self) -> MutexGuard<Connection> {
        self.conn.lock().unwrap()
    }
}

pub fn response<D: Sized>(
    status: StatusCode,
    headers: HeaderMap,
    body: D,
    ctx: &ServerContext,
) -> Response<Body>
where
    Body: From<D>,
{
    let mut response: Response<Body> = Response::default();
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    if ctx.args.production {
        response.headers_mut().insert(
            "Strict-Transport-Security",
            HeaderValue::from_static("max-age=604800; preload"), // 1 week.
        );
    }
    *response.body_mut() = Body::from(body);
    response
}

pub fn response_json<D>(status: StatusCode, body: D, ctx: &ServerContext) -> Response<Body>
where
    D: serde::Serialize,
    Body: From<D>,
{
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", HeaderValue::from_static("application/json"));
    response(status, headers, body, ctx)
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
    let mut posts = { Post::list(&ctx.conn_lock()).unwrap() };
    posts
        .iter_mut()
        .map(|p| {
            crate::md::sanitize_preview(p);
            post_to_html(p, true)
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
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

fn content_type(headers: &mut HeaderMap, content_type: &str) {
    let val = HeaderValue::from_str(content_type).unwrap();
    headers.insert("Content-Type", val);
}

fn cache_control(headers: &mut HeaderMap) {
    let val = HeaderValue::from_static("public, max-age=600");
    headers.insert("Cache-Control", val);
}

async fn get_style(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::html::minify(include_str!("static/style.css"));
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/css");
    cache_control(&mut headers);
    response(StatusCode::OK, headers, body, &ctx)
}

async fn get_script(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::html::minify(include_str!("static/script.js"));
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/javascript");
    cache_control(&mut headers);
    response(StatusCode::OK, headers, body, &ctx)
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
    let post = Post::get(&ctx.conn_lock(), id);
    let post = match post {
        Ok(post) => post,
        Err(_) => return not_found(State(ctx.clone())).await,
    };
    let extra_head = &ctx.args.extra_head;
    let title = crate::md::extract_html_title(&post);
    let settings = PageSettings::new(&title, false, false, Top::GoHome, extra_head);
    let delete_button = indoc::formatdoc! {r#"
        <div class='center medium-text' style='font-weight: bold;'>
            <p>Are you sure you want to delete this post? This action cannot be undone.</p>
            <form action='/posts/delete/{id}' method='post'>
                <button type='submit'>delete</button>
            </form>
            <br>
        </div>
    "#};
    let body = format!("{}\n{}", delete_button, post_to_html(&post, false));
    let body = page(&ctx, &settings, &body);
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

async fn get_edit(
    State(ctx): State<ServerContext>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let post = Post::get(&ctx.conn_lock(), id);
    let post = match post {
        Ok(post) => post,
        Err(_) => return not_found(State(ctx)).await,
    };
    let title = crate::md::extract_html_title(&post);
    let title = format!("Edit '{title}'");
    let body = crate::html::edit_post_form(&post);
    let settings = PageSettings::new(
        &title,
        is_logged_in,
        false,
        Top::GoBack,
        &ctx.args.extra_head,
    );
    let body = page(&ctx, &settings, &body);
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

async fn get_post(
    State(ctx): State<ServerContext>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let post = Post::get(&ctx.conn_lock(), id);
    let post = match post {
        Ok(post) => post,
        Err(_) => return not_found(State(ctx)).await,
    };
    let title = crate::md::extract_html_title(&post);
    let author = &ctx.args.full_name;
    let created = &post.created;
    let updated = &post.updated;
    let extra_head = indoc::formatdoc! {r#"
        <meta property='article:author' content='{author}'/>
        <meta property='article:published_time' content='{created}'/>
        <meta property='article:modified_time' content='{updated}'/>
        {}
    "#, ctx.args.extra_head};
    let settings = PageSettings::new(&title, is_logged_in, false, Top::GoHome, &extra_head);
    let mut body = post_to_html(&post, false);
    if is_logged_in {
        body = format!("{}\n{body}", crate::html::edit_post_buttons(&ctx, &post));
    }
    let body = page(&ctx, &settings, &body);
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
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
    response::<String>(StatusCode::NOT_FOUND, HeaderMap::new(), body, &ctx)
}

async fn get_login(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::html::login(&ctx, None);
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
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
            Err(response::<String>(
                StatusCode::UNAUTHORIZED,
                HeaderMap::new(),
                body,
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
    Post::delete(&ctx.conn_lock(), id).unwrap();
    Ok(Redirect::to("/"))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EditPostForm {
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
    let input = String::from_utf8(bytes).unwrap();
    let publish = input.contains("publish=Publish");
    let form = serde_urlencoded::from_str::<EditPostForm>(&input).unwrap();
    let content = crate::md::auto_autolink(&form.content);
    let created = match Post::get(&ctx.conn_lock(), id) {
        Ok(post) => post.created,
        Err(_) => Utc::now(),
    };
    let post = Post {
        id,
        created,
        updated: Utc::now(),
        content,
    };
    if publish {
        let post = post.update(&ctx.conn_lock());
        if post.is_err() {
            return response(
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                format!("Failed to update post: {}", post.err().unwrap()),
                &ctx,
            );
        };
        let mut headers = HeaderMap::new();
        let url = format!("/posts/{}", id);
        headers.insert("Location", HeaderValue::from_str(&url).unwrap());
        response(StatusCode::SEE_OTHER, headers, "", &ctx)
    } else {
        let preview = crate::html::post_to_html(&post, false);
        let body = page(&ctx, &settings, &preview);
        response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
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
    let input = String::from_utf8(bytes).unwrap();
    let publish = input.contains("publish=Publish");
    let form = serde_urlencoded::from_str::<AddPostForm>(&input).unwrap();
    let content = crate::md::auto_autolink(&form.content);
    if publish {
        let now = Utc::now();
        let post_id = Post::insert(&ctx.conn_lock(), now, now, &content);
        let post_id = if let Ok(post_id) = post_id {
            post_id
        } else {
            return response(
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                "Failed to insert post",
                &ctx,
            );
        };
        let mut headers = HeaderMap::new();
        let url = format!("/posts/{}", post_id);
        headers.insert("Location", HeaderValue::from_str(&url).unwrap());
        response(StatusCode::SEE_OTHER, headers, "", &ctx)
    } else {
        let post = Post {
            id: 0,
            created: Utc::now(),
            updated: Utc::now(),
            content,
        };
        let preview = crate::html::post_to_html(&post, false);
        let body = page(&ctx, &settings, &preview);
        response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
    }
}

async fn get_webfinger(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::ap::webfinger(&ctx);
    let body = match body {
        Some(body) => body,
        None => return not_found(State(ctx)).await,
    }
    .to_string();
    let mut headers = HeaderMap::new();
    headers.insert(
        "Content-Type",
        HeaderValue::from_static("application/jrd+json; charset=utf-8"),
    );
    response::<String>(StatusCode::OK, headers, body, &ctx)
}

pub fn app(ctx: ServerContext) -> Router {
    let router = Router::new()
        .route("/", get(get_posts))
        .route("/posts/delete/{id}", get(get_delete))
        .route("/posts/delete/{id}", post(post_delete))
        .route("/posts/edit/{id}", get(get_edit))
        .route("/posts/edit/{id}", post(post_edit))
        .route("/posts/add", post(post_add))
        .route("/login", get(get_login))
        .route("/login", post(post_login))
        .route("/logout", get(get_logout))
        .route("/posts/{id}", get(get_post))
        .route("/static/style.css", get(get_style))
        .route("/static/script.js", get(get_script))
        .route("/.well-known/webfinger", get(get_webfinger));
    let router = router.fallback(not_found);
    let router = crate::api::routes(&router);
    router.with_state(ctx)
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
