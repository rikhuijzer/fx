mod ap;
mod api;
pub mod data;
mod files;
mod html;
mod md;
pub mod serve;
mod settings;

use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct ServeArgs {
    #[arg(long, env = "FX_PRODUCTION")]
    pub production: bool,
    #[arg(long, env = "FX_PORT", default_value = "3000")]
    pub port: u16,
    #[arg(long, env = "FX_DATABASE_PATH", default_value = "/data/db.sqlite")]
    pub database_path: String,
    /// The username for the admin interface as well as for WebFinger.
    #[arg(long, env = "FX_USERNAME", default_value = "admin")]
    pub username: String,
    /// The password for the admin interface.
    #[arg(long, env = "FX_PASSWORD")]
    pub password: Option<String>,
    /// The domain name of the website.
    #[arg(long, env = "FX_DOMAIN", default_value = "")]
    pub domain: String,
    /// The language of the website.
    #[arg(long, env = "FX_HTML_LANG", default_value = "en")]
    pub html_lang: String,
    /// Content that is added to the `<head>` tag of the HTML page.
    #[arg(long, env = "FX_EXTRA_HEAD", default_value = "")]
    pub extra_head: String,
}
