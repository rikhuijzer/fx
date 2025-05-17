//! Discovery protocols such as sitemap.xml, rss and robots.
use crate::data::Post;
use crate::serve::ServerContext;
use crate::serve::content_type;
use crate::serve::response;
use crate::settings::Settings;
use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::Response;
use axum::http::StatusCode;
use axum::routing::get;

fn rfc822_datetime(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

fn xml_header() -> &'static str {
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
}

#[test]
fn test_xml_header() {
    // Ensure last symbol is a newline and thus trimmed.
    assert_ne!(xml_header(), xml_header().trim());
}

fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&apos;")
}

#[test]
fn test_escape_xml() {
    assert_eq!(escape_xml("foo&bar"), "foo&amp;bar");
}

async fn rss(ctx: &ServerContext, posts: &[Post]) -> String {
    let settings = Settings::from_db(&*ctx.conn().await).unwrap();
    let site_name = escape_xml(&settings.site_name);
    let author_name = escape_xml(&settings.author_name);
    let base = ctx.base_url();
    let mut body = String::new();
    body.push_str(xml_header());
    body.push_str("<rss version=\"2.0\">\n");
    body.push_str("<channel>\n");
    body.push_str(&format!("<title>{site_name}</title>\n"));
    body.push_str(&format!("<link>{base}</link>\n"));
    body.push_str(&format!(
        "<description>Posts by {author_name}</description>\n"
    ));
    for post in posts {
        let title = escape_xml(&crate::md::extract_html_title(post));
        let description = escape_xml(&crate::md::extract_html_description(post));
        let url = format!("{base}/posts/{}", post.id);
        let created = rfc822_datetime(&post.created);
        let entry = format!(
            "
            <item>
            <title>{title}</title>
            <link>{url}</link>
            <guid>{url}</guid>
            <pubDate>{created}</pubDate>
            <description>{description}</description>
            </item>
            "
        );
        body.push_str(&entry);
    }
    body.push_str("</channel>\n");
    body.push_str("</rss>\n");
    crate::html::minify(&body)
}

async fn get_rss(State(ctx): State<ServerContext>) -> Response<Body> {
    let posts = Post::list(&*ctx.conn().await).unwrap();
    let body = rss(&ctx, &posts).await;
    let mut headers = HeaderMap::new();
    // Forces download in Firefox unfortunately:
    // https://www.petefreitag.com/blog/content-type-xml-feeds/
    content_type(&mut headers, "application/rss+xml; charset=utf-8");
    response(StatusCode::OK, headers, body, &ctx)
}

async fn get_robots(State(ctx): State<ServerContext>) -> Response<Body> {
    let base = ctx.base_url();
    let sitemap_url = format!("{base}/sitemap.xml");
    let body = format!(
        "
        User-agent: *
        Disallow:
        Sitemap: {sitemap_url}
        "
    );
    let body = crate::html::minify(&body);
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/plain; charset=utf-8");
    response(StatusCode::OK, headers, body, &ctx)
}

fn w3_datetime(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn sitemap(ctx: &ServerContext, posts: &[Post]) -> String {
    let mut body = String::new();
    body.push_str(xml_header());
    body.push_str("<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
    let base = ctx.base_url();
    body.push_str(&format!("<url><loc>{base}/</loc></url>\n"));
    for post in posts {
        let url = format!("{}/posts/{}", base, post.id);
        let updated = w3_datetime(&post.updated);
        let entry = format!(
            "
            <url>
            <loc>{url}</loc>
            <lastmod>{updated}</lastmod>
            </url>
            "
        );
        body.push_str(&entry);
    }
    body.push_str("</urlset>\n");
    crate::html::minify(&body)
}

async fn get_sitemap(State(ctx): State<ServerContext>) -> Response<Body> {
    let posts = Post::list(&*ctx.conn().await).unwrap();
    let body = sitemap(&ctx, &posts);
    let mut headers = HeaderMap::new();
    content_type(&mut headers, "text/xml");
    response(StatusCode::OK, headers, body, &ctx)
}

pub fn routes(router: &Router<ServerContext>) -> Router<ServerContext> {
    router
        .clone()
        .route("/feed.xml", get(get_rss))
        .route("/robots.txt", get(get_robots))
        .route("/sitemap.xml", get(get_sitemap))
}
