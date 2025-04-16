use crate::data::Post;
use crate::serve::ServerContext;

pub struct HtmlCtx {
    is_logged_in: bool,
}

impl HtmlCtx {
    pub fn new(is_logged_in: bool) -> Self {
        Self { is_logged_in }
    }
}

pub trait ToHtml {
    fn to_html(&self, hctx: &HtmlCtx) -> String;
}

impl ToHtml for Post {
    fn to_html(&self, hctx: &HtmlCtx) -> String {
        let dots = if hctx.is_logged_in { "..." } else { "" };
        indoc::formatdoc! {"
        <div class='post' hx-boost='true'>
            <div class='post-header'>
                <div class='created_at'>{}</div>
                <div class='dots'>{}</div>
            </div>
            <a class='unstyled-link' href='/p/{}'>
                <div class='content'>{}</div>
            </a>
        </div>
        ", self.created_at, dots, self.id, self.content}
    }
}

fn htmx() -> &'static str {
    r#"
    <script src="https://unpkg.com/htmx.org@2.0.4"
    integrity="sha384-HGfztofotfshcF7+8n44JQL2oJmowVChPTg48S+jvZoztPfvwD79OC/LTtG6dMp+" 
    crossorigin="anonymous" defer></script>
    <script src="https://unpkg.com/htmx-ext-preload@2.1.0" 
    integrity="sha384-fkzubQiTB69M7XTToqW6tplvxAOJkqPl5JmLAbumV2EacmuJb8xEP9KnJafk/rg8" 
    crossorigin="anonymous" defer></script>"#
}

pub struct PageSettings {
    title: String,
    is_logged_in: bool,
    show_about: bool,
}

impl PageSettings {
    pub fn new(title: &str, is_logged_in: bool, show_about: bool) -> Self {
        Self {
            title: title.to_string(),
            is_logged_in,
            show_about,
        }
    }
}

pub fn page(ctx: &ServerContext, settings: &PageSettings, body: &str) -> String {
    let htmx = htmx();
    let title = if settings.title.is_empty() {
        "fx".to_string()
    } else {
        format!("{} - fx", settings.title)
    };
    let about = if settings.show_about {
        indoc::formatdoc! {r#"
        <div class="about">
     {}
        </div>
        "#, ctx.args.admin_name }
    } else {
        "".to_string()
    };
    let loginout = if settings.is_logged_in {
        r#"<a class="unstyled-link menu-space" href="/logout">logout</a>"#
    } else {
        r#"<a class="unstyled-link menu-space" href="/login">login</a>"#
    };
    indoc::formatdoc! {
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <link rel="stylesheet" href="/static/style.css">
            <title>{title}</title>
            {htmx}
        </head>
        <body>
            <div class="container">
                <div class="middle">
                    <div class="top">
                        <a class="unstyled-link" href="/">fx</a>
                    </div>
                    {about}
                    {body}
                    <div class="bottom">
                        <a class="unstyled-link menu-space" href="https://github.com/rikhuijzer/fx">source</a>
                        {loginout}
                    </div>
                </div>
            </div>
        </body>
        "#
    }
}

pub fn login(ctx: &ServerContext, error: Option<&str>) -> String {
    let settings = PageSettings::new("login", false, false);
    let error = match error {
        Some(error) => format!("<div style='font-style: italic;'>{error}</div>"),
        None => "".to_string(),
    };
    let body = indoc::formatdoc! {r#"
        <form style="text-align: center;" method="post" action="/login">
            <label for="username">username</label><br>
            <input id="username" name="username" type="text" required/><br>
            <label for="password">password</label><br>
            <input id="password" name="password" type="password" required/><br>
            <br>
            {error}
            <input type="submit" value="login"/>
        </form>
    "#};
    page(ctx, &settings, &body)
}
