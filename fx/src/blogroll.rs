use crate::data::Kv;
use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use crate::serve::ServerContext;
use crate::serve::content_type;
use crate::serve::is_logged_in;
use crate::serve::response;
use crate::settings::InputType;
use crate::settings::text_input;
use axum::Router;
use axum::body::Body;
use axum::extract::Form;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::Response;
use axum::http::StatusCode;
use axum::routing::get;
use axum::routing::post;
use axum_extra::extract::CookieJar;
use chrono::DateTime;
use chrono::Utc;
use fx_rss::Item;
use fx_rss::RssConfig;
use fx_rss::RssFeed;
use serde::Deserialize;

fn show_item(item: &fx_rss::Item) -> Option<String> {
    let feed_name = item.feed_name.clone();
    let pub_date = item.pub_date.clone();
    let pub_date = match pub_date {
        Some(date) => crate::html::show_date(&date),
        None => return None,
    };
    let link = match item.link.clone() {
        Some(link) => link,
        None => return None,
    };
    let title = match item.title.clone() {
        Some(title) => title,
        None => return None,
    };
    Some(format!(
        "
        <span class='blogroll-item' style='font-size: 0.9rem;'>
            {feed_name}: <a href=\"{link}\">{title}</a> ({pub_date})<br>
        </span>
        <br>
        ",
    ))
}

pub struct BlogCache {
    config: RssConfig,
    pub last_updated: DateTime<Utc>,
    pub items: Vec<Item>,
}

impl BlogCache {
    pub async fn new(feeds: Vec<RssFeed>) -> Self {
        let config = fx_rss::RssConfig::new(feeds, 1);
        Self {
            config,
            last_updated: Utc::now(),
            items: vec![],
        }
    }
    pub async fn update(&mut self) {
        let feed = fx_rss::read_rss(&self.config).await;
        let mut items = feed
            .items
            .iter()
            .filter(|item| item.pub_date.is_some())
            .collect::<Vec<_>>();
        items.sort_by(|a, b| b.pub_date.cmp(&a.pub_date));
        self.items = items
            .into_iter()
            .map(|item| item.clone())
            .collect::<Vec<_>>();
        self.last_updated = Utc::now();
    }
}

async fn get_blogroll(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let extra_head = &ctx.args.extra_head;
    let title = "Blogroll";
    let settings = PageSettings::new(title, is_logged_in, false, Top::GoHome, extra_head);

    let last_update = ctx.blog_cache.lock().await.last_updated;
    let items = &ctx.blog_cache.lock().await.items;
    let mut items = items
        .iter()
        .filter(|item| item.pub_date.is_some())
        .collect::<Vec<_>>();
    items.sort_by(|a, b| b.pub_date.cmp(&a.pub_date));
    let items = items
        .iter()
        .filter_map(|item| show_item(item))
        .collect::<Vec<_>>()
        .join("\n");
    let last_update = crate::html::show_date(&last_update);
    let settings_link = if is_logged_in {
        "<a href='/blogroll/settings' class='unstyled-link'>⚙️ Settings</a>"
    } else {
        // Pushes the other element to the right.
        "<span></span>"
    };
    let body = format!(
        "
        <div style='display: flex; justify-content: space-between; font-size: 0.8rem; \
          margin-bottom: 1rem;'>
            {settings_link}
            <div style='text-align: right;'>
                last update: {last_update}
            </div>
        </div>
        {items}
        "
    );
    let body = page(&ctx, &settings, &body).await;
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/html");
    response(StatusCode::OK, headers, body, &ctx)
}

async fn get_settings(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx).await;
    }
    let key = crate::data::BLOGROLL_SETTINGS_KEY;
    let settings = match Kv::get(&*ctx.conn().await, key) {
        Ok(settings) => settings,
        Err(e) => {
            let msg = "Could not get settings from database";
            tracing::error!("{msg}: {e}");
            return crate::serve::internal_server_error(&ctx, msg).await;
        }
    };
    let settings = String::from_utf8(settings).unwrap();
    let style = "margin-top: 5vh; width: 80%";
    let body = format!(
        "
        <style>
            form {{
                margin-left: 1% !important;
                margin-right: 1% !important;
                width: 98% !important;
            }}
            textarea {{
                width: 96% !important;
                font-size: 0.8rem !important;
            }}
        </style>
        <form class='margin-auto' style='{style}' \
          method='post' action='/blogroll/settings'>
            {}
            <input style='margin-left: 0;' type='submit' value='Save'/>
        </form>
        ",
        text_input(
            InputType::Textarea,
            "blogroll_feeds",
            "Feeds",
            &settings,
            "One feed URL per line. For example,
              <pre><code>https://simonwillison.net/atom/everything/</code></pre>
              ",
            true,
        ),
    );
    let page_settings =
        PageSettings::new("Blogroll Settings", is_logged_in, false, Top::GoHome, "");
    let body = page(&ctx, &page_settings, &body).await;
    response(StatusCode::OK, HeaderMap::new(), body, &ctx)
}

#[derive(Debug, Deserialize)]
pub struct BlogrollSettings {
    blogroll_feeds: String,
}

async fn post_settings(
    State(ctx): State<ServerContext>,
    jar: CookieJar,
    Form(form): Form<BlogrollSettings>,
) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    if !is_logged_in {
        return crate::serve::unauthorized(&ctx).await;
    }
    let key = crate::data::BLOGROLL_SETTINGS_KEY;
    let conn = &*ctx.conn().await;
    Kv::insert(conn, key, form.blogroll_feeds.as_bytes()).unwrap();
    crate::trigger::trigger_github_backup(&ctx).await;
    crate::serve::see_other(&ctx, "/blogroll")
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router
        .clone()
        .route("/blogroll", get(get_blogroll))
        .route("/blogroll/settings", get(get_settings))
        .route("/blogroll/settings", post(post_settings))
}
