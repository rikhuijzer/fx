use crate::data::Kv;
use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use crate::serve::ServerContext;
use crate::serve::is_logged_in;
use crate::serve::response;
use axum::Form;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::Response;
use axum::http::StatusCode;
use axum::routing::get;
use axum::routing::post;
use axum_extra::extract::CookieJar;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    pub site_name: String,
    pub author_name: String,
    pub about: String,
}

impl Settings {
    pub fn from_db(conn: &Connection) -> rusqlite::Result<Self> {
        let site_name = Kv::get(conn, "site_name")?;
        let author_name = Kv::get(conn, "author_name")?;
        let about = Kv::get(conn, "about")?;
        Ok(Self {
            site_name: String::from_utf8(site_name).unwrap(),
            author_name: String::from_utf8(author_name).unwrap(),
            about: String::from_utf8(about).unwrap(),
        })
    }
    pub fn set_about(conn: &Connection, about: &str) -> rusqlite::Result<()> {
        Kv::insert(conn, "about", about.as_bytes())?;
        Ok(())
    }
}

enum InputType {
    Text,
    Textarea,
}

fn text_input(
    input_type: InputType,
    name: &str,
    label: &str,
    value: &str,
    description: &str,
    required: bool,
) -> String {
    let required = if required { "required" } else { "" };
    let input = match input_type {
        InputType::Text => format!(
            "
            <input id='{name}' name='{name}' \
            style='width: 100%; margin-top: 0.5rem; margin-bottom: 0.2rem;' \
            type='text' value='{value}' {required}/><br>
            "
        ),
        InputType::Textarea => format!(
            "
            <textarea id='{name}' name='{name}' rows='7' \
            style='width: 100%; margin-top: 0.5rem; margin-bottom: 0.2rem;' \
            {required}>{value}</textarea><br>
            "
        ),
    };
    format!(
        "
        <label for='{name}'>{label}</label><br>
        {input}
        <span style='font-size: 0.8rem; line-height: 1.2; display: inline-block;'>
            {description}
        </span><br>
        <br>
        "
    )
}

async fn get_settings(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx).await;
    }
    let settings = match Settings::from_db(&*ctx.conn().await) {
        Ok(settings) => settings,
        Err(e) => {
            let msg = "Could not get settings from database";
            tracing::error!("{msg}: {e}");
            return crate::serve::internal_server_error(&ctx, msg).await;
        }
    };
    let style = "margin-top: 5vh; width: 80%;";
    let site_name = &settings.site_name;
    let site_name = crate::html::escape_single_quote(site_name);
    let about_description = format!(
        "This is shown below the author name on the front page. This field supports {}.",
        crate::md::markdown_link()
    );
    let body = format!(
        "
        <form class='margin-auto' style='{style}' \
          method='post' action='/settings'>
            {}
            {}
            {}
            <input style='margin-left: 0;' type='submit' value='Save'/>
        </form>
        ",
        text_input(
            InputType::Text,
            "site_name",
            "Site Name",
            &site_name,
            "This is shown in the title of the page.",
            true,
        ),
        text_input(
            InputType::Text,
            "author_name",
            "Author Name",
            &settings.author_name,
            "This is shown at the homepage and in some other places.",
            true,
        ),
        text_input(
            InputType::Textarea,
            "about",
            "About",
            &settings.about,
            &about_description,
            false,
        )
    );
    let page_settings = PageSettings::new("Settings", is_logged_in, false, Top::GoHome, "");
    let body = page(&ctx, &page_settings, &body).await;
    response(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

async fn post_settings(
    State(ctx): State<ServerContext>,
    jar: CookieJar,
    Form(form): Form<Settings>,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx).await;
    }
    let conn = &*ctx.conn().await;
    Kv::insert(conn, "site_name", form.site_name.as_bytes()).unwrap();
    Kv::insert(conn, "author_name", form.author_name.as_bytes()).unwrap();
    Kv::insert(conn, "about", form.about.as_bytes()).unwrap();
    let args = crate::trigger::TriggerArgs::new(&ctx);
    crate::trigger::trigger_github_backup(&args).await;
    crate::serve::see_other(&ctx, "/")
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router
        .clone()
        .route("/settings", get(get_settings))
        .route("/settings", post(post_settings))
}
