use crate::data::Kv;
use crate::data::Post;
use crate::serve::ServerContext;
use chrono::DateTime;

fn border_style(width: u64) -> String {
    format!(
        "border-bottom: {}px solid var(--border); border-radius: 0px;",
        width
    )
}

pub fn escape_single_quote(s: &str) -> String {
    s.replace('\'', "&#39;")
}

fn show_date<Tz: chrono::TimeZone>(datetime: &DateTime<Tz>) -> String {
    datetime.date_naive().to_string()
}

const SET_LEAVE_CONFIRMATION: &str = "window.onbeforeunload = () => true;";
const UNSET_LEAVE_CONFIRMATION: &str = "window.onbeforeunload = null;";

fn turn_title_into_link(post: &Post, html: &str) -> String {
    let html = html.trim();
    let title = html.split("\n").next().unwrap();
    let rest = html.split("\n").skip(1).collect::<Vec<&str>>().join("\n");
    if title.starts_with("# ") {
        let title = title.trim_start_matches("# ");
        format!(
            "<h1><a href='/posts/{}' class='unstyled-link'>{}</a></h1>\n{}",
            post.id, title, rest
        )
    } else {
        html.to_string()
    }
}

pub fn post_to_html(post: &Post, is_preview: bool) -> String {
    // Not wrapping the full post in a `href` because that prevents text
    // selection. I've tried all kinds of workarounds with putting a `position:
    // relative` object in front of the link with `z-index`, but that didn't
    // work. Either the area was clickable or the text was selectable but not
    // both.
    let md = if is_preview {
        turn_title_into_link(post, &post.content)
    } else {
        post.content.clone()
    };
    let html = crate::md::content_to_html(&md);
    let style = if is_preview { &border_style(1) } else { "" };
    let updated = if post.created == post.updated || is_preview {
        ""
    } else {
        &format!(
            "<div class='updated'>last update: {}</div>",
            show_date(&post.updated)
        )
    };
    let post_link = if is_preview {
        format!("<a href='/posts/{}' class='unstyled-link'>", post.id)
    } else {
        "<span>".to_string()
    };
    let post_link_end = if is_preview {
        "</a>".to_string()
    } else {
        "</span>".to_string()
    };
    let post_preview_class = if is_preview { "post-preview" } else { "" };
    indoc::formatdoc! {"
    <div class='post' style='{style}'>
        {post_link}
            <div class='post-header'>
                <div class='created'>{}</div>
                {updated}
            </div>
        {post_link_end}
        <div data-post-id='{}' class='post-content {post_preview_class}'>
        {html}
        </div>
    </div>
    ", show_date(&post.created), post.id}
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
        <a class="button" href="/posts/edit/{id}">
            edit
        </a>
        <a class="button" href="/posts/delete/{id}">
            delete
        </a>
    </div>
    "#}
}

fn add_post_form() -> String {
    format!(
        "
    <form style='width: 100%;' action='/posts/add' method='post'>
        <textarea \
          style='display: block; width: 100%; height: 180px; margin-top: 10px;' \
          class='boxsizing-border' \
          onchange='{SET_LEAVE_CONFIRMATION}' \
          id='content' name='content' placeholder='Your text..'>
        </textarea>
        <br>
        <div style='display: flex; justify-content: flex-end;'>
            <input type='submit' onclick='{UNSET_LEAVE_CONFIRMATION}' \
              name='preview' value='Preview'/>
            <input type='submit' onclick='{UNSET_LEAVE_CONFIRMATION}' \
              name='publish' value='Publish'/>
        </div>
    </form>
    "
    )
    .to_string()
}

pub fn edit_post_form(post: &Post) -> String {
    let id = post.id;
    let content = &post.content;
    format!(
        "
    <form style='width: 100%;' action='/posts/edit/{id}' \
      method='post' onchange='{SET_LEAVE_CONFIRMATION}'>
        <textarea \
          style='display: block; width: 100%; height: 60vh; margin-top: 10px;' \
          class='boxsizing-border' \
          id='content' name='content' placeholder='Your text..'>\n{content}
        </textarea>
        <br>
        <div style='display: flex; justify-content: flex-end;'>
            <input type='submit' onclick='{UNSET_LEAVE_CONFIRMATION}' \
              name='preview' value='Preview'/>
            <input type='submit' onclick='{UNSET_LEAVE_CONFIRMATION}' \
              name='publish' value='Publish'/>
        </div>
    </form>
    "
    )
}

/// Return formatted HTML/CSS that is small and readable.
pub fn minify(page: &str) -> String {
    let mut lines = Vec::new();
    // Whether to minify the current line.
    let mut inside_textarea = false;
    let mut inside_code = false;
    for line in page.lines() {
        let trimmed = line.trim();
        // Don't minify the textarea content or it will effectively modify the
        // textarea content.
        if trimmed.starts_with("<textarea ") {
            inside_textarea = true;
            lines.push(trimmed);
            continue;
        }
        // Don't minify code blocks.
        if trimmed.starts_with("<pre><code") {
            inside_code = true;
            lines.push(trimmed);
            continue;
        }
        if trimmed.starts_with("</code></pre>") {
            inside_code = false;
            lines.push(trimmed);
            continue;
        }
        if trimmed.starts_with("</textarea>") {
            inside_textarea = false;
        }
        if inside_textarea || inside_code {
            lines.push(line);
        } else if !trimmed.is_empty() {
            lines.push(trimmed);
        }
    }
    lines.join("\n")
}

#[test]
fn test_minify() {
    let page = indoc::indoc! {r#"
      <pre><code class="language-rust">x = 1;

    println!("{x}");
    </code></pre>
    "#};
    let expected = indoc::indoc! {r#"
    <pre><code class="language-rust">x = 1;

    println!("{x}");
    </code></pre>
    "#}
    .trim();
    assert_eq!(minify(page), expected);

    let page = indoc::indoc! {r#"
    <textarea id='about'>
    x = 1;

    println!("{x}");
    </textarea>
    "#};
    assert!(minify(page).contains("x = 1;\n\nprintln!"));
}

fn about(ctx: &ServerContext, settings: &PageSettings) -> String {
    let about = Kv::get(&ctx.conn_lock(), "about").unwrap();
    let about = String::from_utf8(about).unwrap();
    let about = crate::md::content_to_html(&about);
    let author_name = Kv::get(&ctx.conn_lock(), "author_name").unwrap();
    let author_name = String::from_utf8(author_name).unwrap();
    let style = "font-size: 0.8rem; padding-top: 0.1rem;";
    let settings_button = if settings.is_logged_in {
        &format!(
            "
        <a href='/settings' class='unstyled-link' style='{style}'>
            ⚙️ Settings
        </a>
        "
        )
    } else {
        ""
    };
    let container_style = "display: flex; justify-content: space-between;";
    let name_style = "font-size: 1.2rem; margin-bottom: 10px; font-weight: bold;";
    format!(
        "
    <div class='introduction' style='padding: 10px; {}'>
        <div style='{container_style}'>
            <div class='full-name' \
                style='{name_style}'>
                {author_name}
            </div>
            <div>
                {settings_button}
            </div>
        </div>
        <div class='about' style='font-size: 0.9rem;'>{about}</div>
    </div>
    ",
        border_style(2),
    )
}

pub fn page(ctx: &ServerContext, settings: &PageSettings, body: &str) -> String {
    let site_name = Kv::get(&ctx.conn_lock(), "site_name").unwrap();
    let site_name = String::from_utf8(site_name).unwrap();
    let site_name = escape_single_quote(&site_name);
    let full_title = if settings.title.is_empty() {
        site_name.clone()
    } else {
        format!("{} - {site_name}", settings.title)
    };
    let about = if settings.show_about {
        about(ctx, settings)
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
                &add_post_form()
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
    let html_lang = &ctx.args.html_lang;
    let extra_head = &settings.extra_head;
    let version = include_str!("version.txt").trim();
    let page = indoc::formatdoc! {
        r#"
        <!DOCTYPE html>
        <html lang='{html_lang}'>
        <head>
            <meta charset='UTF-8'>
            <meta name='viewport' content='width=device-width, initial-scale=1'>
            <link rel='stylesheet' href='/static/style.css'>
            <script src='/static/script.js' defer></script>
            <title>{full_title}</title>
            <meta property='og:site_name' content='{site_name}'/>
            {extra_head}
        </head>
        <body>
            <div class='container'>
                <div class='middle'>
                    {about}
                    <div class='top'>
                        {top}
                    </div>
                    {body}
                    <div class='bottom'>
                        <a class='unstyled-link menu-space' href='https://github.com/rikhuijzer/fx'><u>Running fx</u> version: {version}</a>
                        {loginout}
                    </div>
                </div>
            </div>
        </body>
        "#,
    };
    minify(&page)
}

pub fn login(ctx: &ServerContext, error: Option<&str>) -> String {
    let top = Top::Homepage;
    let settings = PageSettings::new("login", false, false, top, "");
    let error = match error {
        Some(error) => format!("<div style='font-style: italic;'>{error}</div>"),
        None => "".to_string(),
    };
    let style = "text-align: center; margin-top: 15vh;";
    let input_style = "font-size: 1rem;";
    let body = format!(
        "
        <form style='{style}' method='post' action='/login'>
            <input style='{input_style}' id='username' name='username' \
              type='text' placeholder='username' required/><br>
            <input style='{input_style}' id='password' name='password' \
              type='password' placeholder='password' required/><br>
            {error}
            <input style='{input_style}' type='submit' value='login'/>
        </form>
    "
    );
    page(ctx, &settings, &body)
}
