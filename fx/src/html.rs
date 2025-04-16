use crate::data::Post;
use crate::serve::ServerContext;

#[derive(Debug)]
pub struct HtmlCtx {
    #[allow(dead_code)]
    is_logged_in: bool,
    border: bool,
}

impl HtmlCtx {
    pub fn new(is_logged_in: bool, border: bool) -> Self {
        Self {
            is_logged_in,
            border,
        }
    }
}

pub trait ToHtml {
    fn to_html(&self, hctx: &HtmlCtx) -> String;
}

impl ToHtml for Post {
    fn to_html(&self, hctx: &HtmlCtx) -> String {
        let border = if hctx.border {
            "border: 1px solid var(--border);"
        } else {
            ""
        };
        indoc::formatdoc! {"
        <div class='post' style='{border}'>
            <div class='post-header'>
                <div class='created_at'>{}</div>
            </div>
            <a class='unstyled-link' href='/p/{}'>
                <div class='content'>{}</div>
            </a>
        </div>
        ", self.created_at, self.id, self.content}
    }
}

pub enum Top {
    Default,
    Back,
}

pub struct PageSettings {
    title: String,
    is_logged_in: bool,
    show_about: bool,
    top: Top,
}

impl PageSettings {
    pub fn new(title: &str, is_logged_in: bool, show_about: bool, top: Top) -> Self {
        Self {
            title: title.to_string(),
            is_logged_in,
            show_about,
            top,
        }
    }
}

pub fn edit_post_buttons(_ctx: &ServerContext, post: &Post) -> String {
    let id = post.id;
    indoc::formatdoc! {r#"
    <div style="margin-left: auto; display: flex; align-items: center;">
        <button>
            edit
        </button>
        <form style="display: inline-block;" method="post" action="/delete/{id}">
            <input type="submit" value="delete"/>
        </form>
    </div>
    "#}
}

pub fn page(ctx: &ServerContext, settings: &PageSettings, body: &str) -> String {
    let title_suffix = &ctx.args.title_suffix;
    let title = if settings.title.is_empty() {
        title_suffix.clone()
    } else {
        format!("{} - {title_suffix}", settings.title)
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
    let top = match settings.top {
        Top::Default => "",
        Top::Back => indoc::indoc! {"
        <form action='/'>
            <input type='submit' value='← back'/>
        </form>
        "},
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
        </head>
        <body>
            <div class="container">
                <div class="middle">
                    <div class="top">
                        {top}
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
    let top = Top::Default;
    let settings = PageSettings::new("login", false, false, top);
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
