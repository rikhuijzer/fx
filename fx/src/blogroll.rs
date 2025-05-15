use crate::serve::ServerContext;
use crate::serve::content_type;
use crate::serve::response;
use crate::serve::is_logged_in;
use crate::html::PageSettings;
use crate::html::Top;
use crate::html::page;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::Response;
use axum::http::StatusCode;
use axum::routing::get;
use fx_rss::RssFeed;
use axum_extra::extract::CookieJar;

fn show_item(item: &fx_rss::Item) -> Option<String> {
    let feed_name = item.feed_name.clone();
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
        <span class='blogroll-item''>
            {feed_name}: <a href=\"{link}\">{title}</a><br>
        </span>
        <br>
        ",
    ))
}

async fn get_blogroll(State(ctx): State<ServerContext>, jar: CookieJar) -> Response<Body> {
    let is_logged_in = is_logged_in(&ctx, &jar);
    let extra_head = &ctx.args.extra_head;
    let title = "Blogroll";
    let settings = PageSettings::new(title, is_logged_in, false, Top::GoHome, extra_head);

    let feeds = vec![
        RssFeed::new("Economist Writing Every Day", "https://economistwritingeveryday.com/feed"),
        RssFeed::new(
        "Pragmatic Engineer",
        "https://blog.pragmaticengineer.com/feed/",
    ), RssFeed::new(
        "Jaan Juurikas",
        "https://rss.beehiiv.com/feeds/IP6TE2kgRb.xml"
    )];
    let config = fx_rss::RssConfig::new(feeds, 1);
    let feed = fx_rss::read_rss(&config).await;
    let mut items = feed.items.iter().filter(|item| item.pub_date.is_some()).collect::<Vec<_>>();
    items.sort_by(|a, b| b.pub_date.cmp(&a.pub_date));
    let body = items
        .iter()
        .filter_map(|item| show_item(item))
        .collect::<Vec<_>>()
        .join("\n");
    let body = page(&ctx, &settings, &body).await;
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/html");
    response(StatusCode::OK, headers, body, &ctx)
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router.clone().route("/blogroll", get(get_blogroll))
}
