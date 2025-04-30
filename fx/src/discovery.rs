//! Discovery protocols such as sitemap.xml, rss and robots.
use crate::data::Post;
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

fn w3_datetime(dt: &chrono::DateTime<chrono::Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn sitemap(ctx: &ServerContext, posts: &[Post]) -> String {
    let mut body = String::new();
    body.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
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
    router.clone().route("/sitemap.xml", get(get_sitemap))
}
