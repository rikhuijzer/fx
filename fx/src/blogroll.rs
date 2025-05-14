use crate::serve::ServerContext;
use crate::serve::content_type;
use crate::serve::response;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::Response;
use axum::http::StatusCode;
use axum::routing::get;
use fx_rss::RssFeed;

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
        {feed_name}: <a href=\"{link}\">{title}</a><br>
        <br>
        ",
    ))
}

async fn get_blogroll(State(ctx): State<ServerContext>) -> Response<Body> {
    let feeds = vec![RssFeed::new(
        "Pragmatic Engineer",
        "https://blog.pragmaticengineer.com/feed/",
    )];
    let config = fx_rss::RssConfig::new(feeds, 1);
    let feed = fx_rss::read_rss(&config).await;
    let body = feed
        .items
        .iter()
        .filter_map(|item| show_item(item))
        .collect::<Vec<_>>()
        .join("\n");
    let body = crate::html::minify(&body);
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/html");
    response(StatusCode::OK, headers, body, &ctx)
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router.clone().route("/blogroll", get(get_blogroll))
}
