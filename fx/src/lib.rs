mod ap;
mod api;
pub mod blogroll;
pub mod data;
mod discovery;
mod files;
pub mod html;
mod md;
mod search;
pub mod serve;
mod settings;
mod trigger;

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

    /// The token for triggering GitHub Actions.
    #[arg(long, env = "FX_TRIGGER_TOKEN")]
    pub trigger_token: Option<String>,
    /// The owner and repository for triggering GitHub Actions.
    #[arg(long, env = "FX_TRIGGER_OWNER_REPO")]
    pub trigger_owner_repo: Option<String>,
    /// The host on which the workflow is triggered (defaults to GitHub).
    #[arg(long, env = "FX_TRIGGER_HOST", default_value = "https://github.com")]
    pub trigger_host: String,
    /// The branch in which the workflow is triggered (defaults to `main`).
    #[arg(long, env = "FX_TRIGGER_BRANCH", default_value = "main")]
    pub trigger_branch: String,
    /// The workflow ID (filename) that is triggered (defaults to `backup.yml`).
    #[arg(long, env = "FX_TRIGGER_WORKFLOW_ID", default_value = "backup.yml")]
    pub trigger_workflow_id: String,
}
