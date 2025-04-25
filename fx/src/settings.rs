use crate::data::Kv;
use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use crate::serve::ServerContext;
use crate::serve::is_logged_in;
use crate::serve::response;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::Response;
use axum::http::StatusCode;
use axum::routing::get;
use axum_extra::extract::CookieJar;
use rusqlite::Connection;
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    pub title_suffix: String,
    pub full_name: String,
}

impl Settings {
    fn from_db(conn: &Connection) -> rusqlite::Result<Self> {
        let title_suffix = Kv::get(conn, "title_suffix")?;
        let full_name = Kv::get(conn, "full_name")?;
        Ok(Self {
            title_suffix: String::from_utf8(title_suffix).unwrap(),
            full_name: String::from_utf8(full_name).unwrap(),
        })
    }
}

fn text_input(name: &str, label: &str, placeholder: &str, description: &str) -> String {
    format!(
        "
    <label for='{name}'>{label}</label><br>
    <input id='{name}' name='{name}' \
      style='width: 100%; margin-top: 0.5rem; margin-bottom: 0.2rem;' \
      type='text' placeholder='{placeholder}' required/><br>
    <span style='font-size: 0.8rem;'>{description}</span><br>
    <br>
    "
    )
}

async fn get_settings(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx);
    }
    let settings = match Settings::from_db(&ctx.conn_lock()) {
        Ok(settings) => settings,
        Err(e) => {
            let msg = "Could not get settings from database";
            tracing::error!("{msg}: {e}");
            return crate::serve::internal_server_error(&ctx, msg);
        }
    };
    let style = "margin-top: 5vh; width: 80%; \
      margin-left: auto; margin-right: auto;";
    let body = format!(
        "
        <form style='{style}' method='post' action='/settings'>
            {}
            {}
            <input type='submit' value='save'/>
        </form>
    ",
        text_input(
            "title-suffix",
            "Title suffix",
            "unset",
            "For example, 'My Blog'"
        ),
        text_input("full-name", "Full name", "unset", "For example, 'John Doe'")
    );
    let page_settings = PageSettings::new("Settings", is_logged_in, false, Top::GoHome, "");
    let body = page(&ctx, &page_settings, &body);
    response(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router.clone().route("/settings", get(get_settings))
}
