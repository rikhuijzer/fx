pub mod data;
pub mod html;
mod md;
pub mod serve;

use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct ServeArgs {
    #[arg(long, env = "FX_PRODUCTION")]
    pub production: bool,
    #[arg(long, env = "FX_PORT", default_value = "3000")]
    pub port: u16,
    #[arg(long, env = "FX_DATABASE_PATH", default_value = "/data/db.sqlite")]
    pub database_path: String,
    #[arg(long, env = "FX_ADMIN_USERNAME", default_value = "admin")]
    pub admin_username: String,
    #[arg(long, env = "FX_ADMIN_PASSWORD")]
    pub admin_password: Option<String>,
    /// The website title.
    #[arg(long, env = "FX_TITLE_SUFFIX", default_value = "fx")]
    pub title_suffix: String,
    /// The full name that is shown on top of the main page.
    #[arg(long, env = "FX_FULL_NAME", default_value = "John Doe")]
    pub full_name: String,
    /// The about text that is shown below the full name on the front page.
    #[arg(long, env = "FX_ABOUT", default_value = "")]
    pub about: String,
    /// The language of the website.
    #[arg(long, env = "FX_HTML_LANG", default_value = "en")]
    pub html_lang: String,
    /// Content that is added to the `<head>` tag of the HTML page.
    #[arg(long, env = "FX_EXTRA_HEAD", default_value = "")]
    pub extra_head: String,
}
