//! ActivityPub

use crate::serve::ServerContext;
use serde_json::Value;
use serde_json::json;

/// Returns a JSON object that can be used as WebFinger response.
///
/// And do some basic verification via <https://webfinger.net/>.
pub fn webfinger(ctx: &ServerContext) -> Option<Value> {
    let domain = match &ctx.args.domain {
        Some(domain) => domain,
        None => return None,
    };
    let username = &ctx.args.username;
    let domain = domain.trim_matches('/');
    let domain = domain.replace("http://", "");
    let domain = domain.replace("https://", "");
    let domain = domain.trim();
    Some(json!({
        "subject": format!("acct:{username}@{domain}"),
        "aliases": [
            format!("https://{domain}"),
        ],
        "links": [
            {
                "rel": "http://webfinger.net/rel/profile-page",
                "type": "text/html",
                "href": format!("https://{domain}"),
            },
        ],
    }))
}
