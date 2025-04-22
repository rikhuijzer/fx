mod ap;
pub mod data;
pub mod html;
mod md;
pub mod serve;

#[derive(Clone, Debug)]
pub struct ServeArgs {
    pub production: bool,
    pub port: u16,
    pub database_path: String,
    /// The username for the admin interface as well as for WebFinger.
    pub username: String,
    /// The password for the admin interface.
    pub password: Option<String>,
    /// The website title.
    pub title_suffix: String,
    /// The website domain name, for example "example.com".
    ///
    /// Required for WebFinger.
    pub domain: Option<String>,
    /// The full name that is shown on top of the main page.
    pub full_name: String,
    /// The about text that is shown below the full name on the front page.
    pub about: String,
    /// The language of the website.
    pub html_lang: String,
    /// Content that is added to the `<head>` tag of the HTML page.
    pub extra_head: String,
}
