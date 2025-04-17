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
    /// The website title.
    #[arg(long, env = "FX_TITLE_SUFFIX", default_value = "fx")]
    pub title_suffix: String,
    /// The full name that is shown on top of the main page.
    #[arg(long, env = "FX_ADMIN_NAME", default_value = "John Doe")]
    pub admin_name: String,
    #[arg(long, env = "FX_ADMIN_PASSWORD")]
    pub admin_password: Option<String>,
}
