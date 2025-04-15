pub mod data;
pub mod html;
pub mod serve;

use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct ServeArgs {
    #[arg(long, env = "PRODUCTION")]
    pub production: bool,
    #[arg(long, env = "PORT", default_value = "3000")]
    pub port: u16,
    #[arg(long, env = "DATABASE_PATH", default_value = "/data/db.sqlite")]
    pub database_path: String,
    #[arg(long, env = "ADMIN_USERNAME", default_value = "admin")]
    pub admin_username: String,
    #[arg(long, env = "ADMIN_PASSWORD")]
    pub admin_password: Option<String>,
}
