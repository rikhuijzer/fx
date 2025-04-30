//! ActivityPub

use crate::serve::ServerContext;
use serde_json::Value;
use serde_json::json;

/// Returns a JSON object that can be used as WebFinger response.
///
/// And do some basic verification via <https://webfinger.net/>.
pub async fn webfinger(ctx: &ServerContext) -> Option<Value> {
    let domain = ctx.base_url();
    let username = &ctx.args.username;
    Some(json!({
        "subject": format!("acct:{username}@{domain}"),
        "aliases": [
            domain,
        ],
        "links": [
            {
                "rel": "http://webfinger.net/rel/profile-page",
                "type": "text/html",
                "href": domain,
            },
        ],
    }))
}
