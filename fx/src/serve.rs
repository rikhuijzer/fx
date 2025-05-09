use crate::ServeArgs;
use crate::data;
use crate::data::Kv;
use crate::data::Post;
use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use crate::html::wrap_post_content;
use axum::Form;
use axum::Router;
use axum::body::Body;
use axum::extract::DefaultBodyLimit;
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
use tokio::sync::Mutex;
use tokio::sync::MutexGuard;

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
    pub async fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().await
    }
    /// Returns the base URL of the server.
    ///
    /// For example, if the domain is "example.com", the base URL will be
    /// "https://example.com".
    pub fn base_url(&self) -> String {
        if self.args.domain.is_empty() {
            "".to_string()
        } else {
            let domain = &self.args.domain;
            let domain = domain.trim();
            let domain = domain.trim_end_matches('/');
            format!("https://{domain}")
        }
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

pub async fn error(
    ctx: &ServerContext,
    status: StatusCode,
    title: &str,
    msg: &str,
) -> Response<Body> {
    let body = msg.to_string();
    let headers = HeaderMap::new();
    let settings = PageSettings::new(title, true, false, Top::GoHome, "");
    let body = format!(
        "
        <div style='text-align: center;'>
            <h1>{title}</h1>
            <p>{body}</p>
        </div>
        "
    );
    let body = page(ctx, &settings, &body).await;
    response(status, headers, body, ctx)
}

pub async fn internal_server_error(ctx: &ServerContext, msg: &str) -> Response<Body> {
    error(
        ctx,
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal Server Error",
        msg,
    )
    .await
}

pub async fn unauthorized(ctx: &ServerContext) -> Response<Body> {
    error(
        ctx,
        StatusCode::UNAUTHORIZED,
        "Unauthorized",
        "Not logged in",
    )
    .await
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

pub fn is_logged_in(ctx: &ServerContext, jar: &CookieJar) -> bool {
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

async fn list_posts(ctx: &ServerContext, _is_logged_in: bool) -> String {
    let mut posts = { Post::list(&*ctx.conn().await).unwrap() };
    posts
        .iter_mut()
        .map(|post| {
            crate::md::preview(post, 600);
            wrap_post_content(post, true)
        })
        .collect::<Vec<String>>()
        .join("\n")
}

async fn get_posts(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let extra_head = &ctx.args.extra_head;
    let settings = PageSettings::new("", is_logged_in, true, Top::Homepage, extra_head);
    let posts = list_posts(&ctx, is_logged_in).await;
    let body = page(&ctx, &settings, &posts).await;
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

pub fn content_type(headers: &mut HeaderMap, content_type: &str) {
    let val = HeaderValue::from_str(content_type).unwrap();
    headers.insert("Content-Type", val);
}

pub fn enable_caching(headers: &mut HeaderMap, max_age: u32) {
    // `must-revalidate` avoids stale responses when disconnected.
    let src = format!("public, max-age={max_age}, must-revalidate");
    let val = HeaderValue::from_str(&src).unwrap();
    headers.insert(hyper::header::CACHE_CONTROL, val);
}

async fn get_style(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::html::minify(include_str!("static/style.css"));
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/css");
    enable_caching(&mut headers, 600);
    response(StatusCode::OK, headers, body, &ctx)
}

async fn get_script(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::html::minify(include_str!("static/script.js"));
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/javascript");
    enable_caching(&mut headers, 600);
    response(StatusCode::OK, headers, body, &ctx)
}

async fn get_katex(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::html::minify(include_str!("static/katex.js"));
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/javascript");
    enable_caching(&mut headers, 600);
    response(StatusCode::OK, headers, body, &ctx)
}

async fn get_nodefer(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::html::minify(include_str!("static/nodefer.js"));
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/javascript");
    enable_caching(&mut headers, 600);
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
    let post = Post::get(&*ctx.conn().await, id);
    let post = match post {
        Ok(post) => post,
        Err(_) => return not_found(State(ctx.clone())).await,
    };
    let extra_head = &ctx.args.extra_head;
    let title = crate::md::extract_html_title(&post);
    let settings = PageSettings::new(&title, false, false, Top::GoHome, extra_head);
    let delete_button = indoc::formatdoc! {r#"
        <div class='medium-text' style='text-align: center; font-weight: bold;'>
            <p>Are you sure you want to delete this post? This action cannot be undone.</p>
            <form action='/posts/delete/{id}' method='post'>
                <button type='submit'>delete</button>
            </form>
            <br>
        </div>
    "#};
    let body = format!("{}\n{}", delete_button, wrap_post_content(&post, false));
    let body = page(&ctx, &settings, &body).await;
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

async fn get_edit(
    State(ctx): State<ServerContext>,
    Path(id): Path<i64>,
    jar: CookieJar,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let post = Post::get(&*ctx.conn().await, id);
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
    let body = page(&ctx, &settings, &body).await;
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

fn iso8601(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

async fn get_post(
    State(ctx): State<ServerContext>,
    Path(id): Path<String>,
    jar: CookieJar,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let id = match id.parse::<i64>() {
        Ok(id) => id,
        Err(_) => return not_found(State(ctx)).await,
    };
    let post = Post::get(&*ctx.conn().await, id);
    let post = match post {
        Ok(post) => post,
        Err(_) => return not_found(State(ctx)).await,
    };
    let title = crate::md::extract_html_title(&post);
    let author = Kv::get(&*ctx.conn().await, "author_name").unwrap();
    let author = String::from_utf8(author).unwrap();
    // Open Graph uses ISO 8601 according to <https://ogp.me/>.
    let created = iso8601(&post.created);
    let updated = iso8601(&post.updated);
    let canonical = format!("{}/posts/{}", &ctx.base_url(), &post.id);
    let extra_head = indoc::formatdoc! {r#"
        <meta property='article:author' content='{author}'/>
        <meta property='article:published_time' content='{created}'/>
        <meta property='article:modified_time' content='{updated}'/>
        <link rel='canonical' href='{canonical}'/>
        {}
    "#, ctx.args.extra_head};
    let settings = PageSettings::new(&title, is_logged_in, false, Top::GoHome, &extra_head);
    let mut body = wrap_post_content(&post, false);
    if is_logged_in {
        body = format!("{}\n{body}", crate::html::edit_post_buttons(&ctx, &post));
    }
    let body = page(&ctx, &settings, &body).await;
    response::<String>(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

pub async fn not_found(State(ctx): State<ServerContext>) -> Response<Body> {
    // Should probably not show the login button at all on 404 pages.
    let is_logged_in = false;
    let body = indoc::indoc! {"
        <div style='text-align: center; margin-top: 100px;'>
            <h1>Not found</h1>
            <p>The page you are looking for does not exist.</p>
        </div>
    "};
    let extra_head = &ctx.args.extra_head;
    let settings = PageSettings::new("not found", is_logged_in, false, Top::GoHome, extra_head);
    let body = page(&ctx, &settings, body).await;
    response::<String>(StatusCode::NOT_FOUND, HeaderMap::new(), body, &ctx)
}

async fn get_login(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::html::login(&ctx, None).await;
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
                body.await,
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
    Post::delete(&*ctx.conn().await, id).unwrap();
    crate::trigger::trigger_github_backup(&ctx).await;
    Ok(Redirect::to("/"))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EditPostForm {
    pub content: String,
}

/// Return a 303 redirect to the given url.
///
/// This is used after a `POST` request to indicate that the resource has been
/// updated and the client should fetch the updated resource with a `GET`
/// request.
pub fn see_other(ctx: &ServerContext, url: &str) -> Response<Body> {
    let mut headers = HeaderMap::new();
    let dst = HeaderValue::from_str(url).unwrap();
    headers.insert("Location", dst);
    response(StatusCode::SEE_OTHER, headers, "", ctx)
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
    let created = match Post::get(&*ctx.conn().await, id) {
        Ok(post) => post.created,
        Err(_) => Utc::now(),
    };
    let post = Post {
        id,
        created,
        updated: Utc::now(),
        content: form.content,
    };
    if publish {
        let post = post.update(&*ctx.conn().await);
        if post.is_err() {
            return response(
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                format!("Failed to update post: {}", post.err().unwrap()),
                &ctx,
            );
        };
        let url = format!("/posts/{}", id);
        crate::trigger::trigger_github_backup(&ctx).await;
        see_other(&ctx, &url)
    } else {
        let preview = crate::html::wrap_post_content(&post, false);
        let body = page(&ctx, &settings, &preview).await;
        response(StatusCode::OK, HeaderMap::new(), body, &ctx)
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
    if publish {
        let now = Utc::now();
        let post_id = Post::insert(&*ctx.conn().await, now, now, &form.content);
        if let Err(_e) = post_id {
            return response(
                StatusCode::INTERNAL_SERVER_ERROR,
                HeaderMap::new(),
                "Failed to insert post",
                &ctx,
            );
        };
        let url = "/?reset_forms=true";
        crate::trigger::trigger_github_backup(&ctx).await;
        see_other(&ctx, url)
    } else {
        let post = Post {
            id: 0,
            created: Utc::now(),
            updated: Utc::now(),
            content: form.content,
        };
        let is_front_page_preview = false;
        let preview = crate::html::wrap_post_content(&post, is_front_page_preview);
        let body = page(&ctx, &settings, &preview).await;
        response(StatusCode::OK, HeaderMap::new(), body, &ctx)
    }
}

async fn get_webfinger(State(ctx): State<ServerContext>) -> Response<Body> {
    let body = crate::ap::webfinger(&ctx).await;
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
        .route("/posts/{id}", get(get_post))
        .route("/login", get(get_login))
        .route("/login", post(post_login))
        .route("/logout", get(get_logout))
        .route("/static/style.css", get(get_style))
        .route("/static/script.js", get(get_script))
        .route("/static/katex.js", get(get_katex))
        .route("/static/nodefer.js", get(get_nodefer))
        .route("/.well-known/webfinger", get(get_webfinger));
    let router = crate::api::routes(&router);
    let router = crate::discovery::routes(&router);
    let router = crate::files::routes(&router);
    let router = crate::search::routes(&router);
    let router = crate::settings::routes(&router);
    let router = router.fallback(not_found);
    // Files larger than this will be rejected during upload.
    let limit = 15 * 1024 * 1024;
    router.with_state(ctx).layer(DefaultBodyLimit::max(limit))
}

/// Return the salt by either generating a new one or reading it from the db.
///
/// Re-using the salt between sessions allows users to keep logged in even when
/// the server restarts.
fn obtain_salt(args: &ServeArgs, conn: &Connection) -> Salt {
    if args.production {
        let salt = data::Kv::get(conn, "salt");
        match salt {
            Ok(salt) => salt.try_into().unwrap(),
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
