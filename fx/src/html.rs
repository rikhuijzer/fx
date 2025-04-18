use crate::data::Post;
use crate::data::SqliteDateTime;
use crate::serve::ServerContext;
use chrono::DateTime;

fn border_style(width: u64) -> String {
    format!(
        "border-bottom: {}px solid var(--border); border-radius: 0px;",
        width
    )
}

fn show_date<Tz: chrono::TimeZone>(datetime: &DateTime<Tz>) -> String {
    datetime.date_naive().to_string()
}

pub fn post_to_html(post: &Post, border: bool) -> String {
    let html = crate::md::to_html(&post.content);
    let style = if border { &border_style(1) } else { "" };
    let updated = if post.created == post.updated {
        ""
    } else {
        &format!(
            "div class='updated'>last update: {}</div>",
            show_date(&post.updated)
        )
    };
    indoc::formatdoc! {"
    <div class='post' style='{style}'>
        <div class='post-header'>
            <div class='created'>{}</div>
            {updated}
        </div>
        <div class='content'>{html}</div>
    </div>
    ", show_date(&post.created)}
}

pub enum Top {
    /// Show the top section for the homepage.
    Homepage,
    /// A button that goes back to the homepage.
    GoHome,
    /// A button that goes back to the previous page.
    ///
    /// This is used after the preview to go back to the post that was being
    /// edited.
    GoBack,
}

pub struct PageSettings {
    title: String,
    is_logged_in: bool,
    show_about: bool,
    top: Top,
    extra_head: String,
}

impl PageSettings {
    pub fn new(
        title: &str,
        is_logged_in: bool,
        show_about: bool,
        top: Top,
        extra_head: &str,
    ) -> Self {
        Self {
            title: title.to_string(),
            is_logged_in,
            show_about,
            top,
            extra_head: extra_head.to_string(),
        }
    }
}

pub fn edit_post_buttons(_ctx: &ServerContext, post: &Post) -> String {
    let id = post.id;
    indoc::formatdoc! {r#"
    <div style="margin-left: auto; display: flex; align-items: center;">
        <a class="button" href="/post/edit/{id}">
            edit
        </a>
        <a class="button" href="/post/delete/{id}">
            delete
        </a>
    </div>
    "#}
}

fn add_post_form() -> &'static str {
    "
    <form style='width: 100%;' action='/post/add' method='post'>
        <textarea \
          style='display: block; width: 100%; height: 100px; margin-top: 10px;' \
          class='boxsizing-border' \
          id='content' name='content' placeholder='Your text..'>
        </textarea>
        <br>
        <div style='display: flex; justify-content: flex-end;'>
            <input type='submit' name='preview' value='Preview'/>
            <input type='submit' name='publish' value='Publish'/>
        </div>
    </form>
    "
}

pub fn edit_post_form(post: &Post) -> String {
    let id = post.id;
    let content = &post.content;
    let created = post.created.to_sqlite();
    let updated = post.updated.to_sqlite();
    format!(
        "
    <form style='width: 100%;' action='/post/edit/{id}' method='post'>
        <label style='font-size: 0.8rem;' for='created'>Created (UTC)</label>
        <input name='created' value='{created}'><br>
        <label style='font-size: 0.8rem;' for='updated'>Updated (UTC)</label>
        <input name='updated' value='{updated}'>
        <textarea \
          style='display: block; width: 100%; height: 60vh; margin-top: 10px;' \
          class='boxsizing-border' \
          id='content' name='content' placeholder='Your text..'>{content}</textarea>
        </textarea>
        <br>
        <div style='display: flex; justify-content: flex-end;'>
            <input type='submit' name='preview' value='Preview'/>
            <input type='submit' name='publish' value='Publish'/>
        </div>
    </form>
    "
    )
}

/// Return formatted HTML that is more readable.
fn pretty_html(page: &str) -> String {
    page.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("\n")
}

pub fn page(ctx: &ServerContext, settings: &PageSettings, body: &str) -> String {
    let title_suffix = &ctx.args.title_suffix;
    let full_title = if settings.title.is_empty() {
        title_suffix.clone()
    } else {
        format!("{} - {title_suffix}", settings.title)
    };
    let about = if settings.show_about {
        format!(
            "
        <div class='introduction' style='padding: 10px; {}'>
            <div class='full-name' \
              style='font-size: 1.2rem; margin-bottom: 10px; font-weight: bold;'>
                {}
            </div>
            <div class='about' style='font-size: 0.9rem;'>{}</div>
        </div>
        ",
            border_style(2),
            ctx.args.full_name,
            ctx.args.about
        )
    } else {
        "".to_string()
    };
    let loginout = if settings.is_logged_in {
        r#"<a class="unstyled-link menu-space" href="/logout">Logout</a>"#
    } else {
        r#"<a class="unstyled-link menu-space" href="/login">Login</a>"#
    };
    let top = match settings.top {
        Top::Homepage => {
            if settings.is_logged_in {
                add_post_form()
            } else {
                ""
            }
        }
        Top::GoHome => indoc::indoc! {"
        <a href='/' class='button'>← back</a>
        "},
        Top::GoBack => indoc::indoc! {r#"
        <noscript>
            // no button because loading back will remove the previous content.
        </noscript>
        <script>
            document.write("<a href='javascript:history.back()' class='button'>← back</a>");
        </script>
        "#},
    };
    let title = &settings.title;
    let html_lang = &ctx.args.html_lang;
    let extra_head = &settings.extra_head;
    let page = indoc::formatdoc! {
        r#"
        <!DOCTYPE html>
        <html lang="{html_lang}">
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <link rel="stylesheet" href="/static/style.css">
            <title>{full_title}</title>
            <meta property="og:site_name" content="{title}"/>
            {extra_head}
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
                        <a class="unstyled-link menu-space" href="https://github.com/rikhuijzer/fx">Source</a>
                        {loginout}
                    </div>
                </div>
            </div>
        </body>
        "#,
    };
    pretty_html(&page)
}

pub fn login(ctx: &ServerContext, error: Option<&str>) -> String {
    let top = Top::Homepage;
    let settings = PageSettings::new("login", false, false, top, "");
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
