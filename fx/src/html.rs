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
            <div class='content'>{}</div>
        </div>
        ", self.created_at, self.content}
    }
}

pub enum Top {
    Homepage,
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
        <a class="button" href="/post/delete/{id}">
            delete
        </a>
    </div>
    "#}
}

fn add_post_form() -> &'static str {
    indoc::indoc! {r#"
    <form style="width: 100%;" action="/post/add" method="post">
        <textarea style="width: 99%; height: 100px;"
          id="content" name="content" placeholder="content"></textarea>
        <br>
        <div style="display: flex; justify-content: flex-end;">
            <input type="submit" value="preview"/>
            <input type="submit" value="publish"/>
        </div>
    </form>
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
        <div class="about" style="margin-bottom: 20px;">
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
        Top::Homepage => {
            if settings.is_logged_in {
                add_post_form()
            } else {
                ""
            }
        }
        Top::Back => indoc::indoc! {"
        <a href='/' class='button'>← back</a>
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
                    {about}
                    <div class="top">
                        {top}
                    </div>
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
    let top = Top::Homepage;
    let settings = PageSettings::new("login", false, false, top);
    let error = match error {
        Some(error) => format!("<div style='font-style: italic;'>{error}</div>"),
        None => "".to_string(),
    };
    let body = indoc::formatdoc! {r#"
        <form style="text-align: center; margin-top: 15vh;" method="post" action="/login">
            <input style="font-size: 1rem;" id="username" name="username" type="text"
               placeholder="username" required/><br>
            <input style="font-size: 1rem;" id="password" name="password" type="password"
               placeholder="password" required/><br>
            {error}
            <input style="font-size: 1rem;" type="submit" value="login"/>
        </form>
    "#};
    page(ctx, &settings, &body)
}
