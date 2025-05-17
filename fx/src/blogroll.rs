use crate::data::Kv;
use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use crate::serve::ServerContext;
use crate::serve::content_type;
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
use chrono::DateTime;
use chrono::Utc;
use fx_rss::Item;
use fx_rss::RssConfig;
use fx_rss::RssFeed;

fn show_item(item: &fx_rss::Item) -> Option<String> {
    let feed_name = item.feed_name.clone();
    let pub_date = match item.pub_date {
        Some(date) => crate::html::show_date(&date),
        None => return None,
    };
    let link = item.link.clone()?;
    let title = item.title.clone()?;
    Some(format!(
        "
        <span class='blogroll-item' style='font-size: 0.9rem;'>
            {feed_name}: <a href=\"{link}\">{title}</a> ({pub_date})
        </span>
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
    pub async fn update(&mut self, ctx: &ServerContext) {
        let key = crate::data::BLOGROLL_SETTINGS_KEY;
        let feeds = Kv::get(&*ctx.conn().await, key);
        if let Ok(feeds) = feeds {
            let feeds = String::from_utf8(feeds).unwrap();
            if feeds.trim().is_empty() {
                self.config.feeds = vec![];
                self.items = vec![];
                self.last_updated = Utc::now();
                return;
            } else {
                let feeds = feeds
                    .split("\n")
                    .map(|line| line.trim())
                    .collect::<Vec<_>>();
                self.config.feeds = feeds.into_iter().map(RssFeed::new).collect::<Vec<_>>();
            }
        }
        let items = self.config.download_items().await;
        let mut items = items
            .iter()
            .filter(|item| item.pub_date.is_some())
            .collect::<Vec<_>>();
        items.sort_by(|a, b| b.pub_date.cmp(&a.pub_date));
        self.items = items.into_iter().cloned().collect::<Vec<_>>();
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
        "<a href='/settings#blogroll_feeds_label' class='unstyled-link'>⚙️ Settings</a>"
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

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router.clone().route("/blogroll", get(get_blogroll))
}
