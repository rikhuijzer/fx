use crate::data::Kv;
use crate::data::Post;
use crate::serve::ServerContext;
use chrono::DateTime;
use chrono::Duration;

fn border_style(width: u64) -> String {
    format!(
        "border-bottom: {}px solid var(--border); border-radius: 0px;",
        width
    )
}

pub fn escape_single_quote(s: &str) -> String {
    s.replace('\'', "&#39;")
}

pub fn show_date<Tz: chrono::TimeZone>(datetime: &DateTime<Tz>) -> String {
    let now = chrono::Utc::now();
    let duration = now.signed_duration_since(datetime.clone());

    if duration < Duration::hours(24) && duration >= Duration::zero() {
        let hours = duration.num_hours();
        if hours == 0 {
            let minutes = duration.num_minutes();
            if minutes == 0 {
                "just now".to_string()
            } else if minutes == 1 {
                "1 minute ago".to_string()
            } else {
                format!("{} minutes ago", minutes)
            }
        } else if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else {
        datetime.date_naive().format("%Y-%m-%d").to_string()
    }
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

/// Automatically set the `id` attribute for headers.
///
/// pulldown-cmark supports header attributes, but markdown-rs does not. That's
/// why we need to fix the html manually. Note that pulldown-cmark also does not
/// automatically set the `id` attribute
/// (https://github.com/pulldown-cmark/pulldown-cmark/issues/700), so some kind
/// of fixing is needed anyway.
fn set_header_id(html: &str) -> String {
    html.lines()
        .map(|line| {
            if line.trim().starts_with("<h") {
                if line.contains(" id=") {
                    return line.to_string();
                }
                // --- is sometimes interpreted as a horizontal rule.
                if line.contains("<hr />") {
                    return line.to_string();
                }
                let title_start = line.find('>').unwrap() + 1;
                let level = line[2..title_start - 1].to_string();
                let title_end = match line.find("</h") {
                    Some(end) => end,
                    None => {
                        tracing::warn!("could not find </h> in {line}");
                        return line.to_string();
                    }
                };
                let title = line[title_start..title_end].to_string();
                let id = title.to_lowercase().replace(' ', "-");
                format!("<h{level} id='{id}'>{title}</h{level}>")
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

#[test]
fn test_set_header_id() {
    let html = indoc::indoc! {r#"
    <h1>Hello</h1>
    <h2>Bar baz</h2>
    <h3 id='quux'>Quux</h3>

    "#};
    let actual = set_header_id(html);
    println!("actual:\n{}", actual);
    let expected = indoc::indoc! {r#"
    <h1 id='hello'>Hello</h1>
    <h2 id='bar-baz'>Bar baz</h2>
    <h3 id='quux'>Quux</h3>

    "#};
    assert_eq!(actual.trim(), expected.trim());

    let html = indoc::indoc! {r#"
        <pre><code>function f(x)
            println(1)
        end
        </code></pre>
        "#}
    .trim();
    assert_eq!(html, set_header_id(html), "code block has changed");

    let html = indoc::indoc! {r#"
        <hr />
        "#}
    .trim();
    assert!(!set_header_id(html).is_empty());
}

pub fn wrap_post_content(post: &Post, is_front_page_preview: bool) -> String {
    // Not wrapping the full post in a `href` because that prevents text
    // selection. I've tried all kinds of workarounds with putting a `position:
    // relative` object in front of the link with `z-index`, but that didn't
    // work. Either the area was clickable or the text was selectable but not
    // both.
    let html = if is_front_page_preview {
        turn_title_into_link(post, &post.content)
    } else {
        post.content.clone()
    };
    let html = if is_front_page_preview {
        // Front page preview is already HTML.
        html
    } else {
        crate::md::content_to_html(&html)
    };
    let html = set_header_id(&html);
    let style = if is_front_page_preview {
        &border_style(1)
    } else {
        ""
    };
    let updated = if post.created == post.updated || is_front_page_preview {
        ""
    } else {
        &format!(
            "<div class='updated'>last update: {}</div>",
            show_date(&post.updated)
        )
    };
    let post_link = if is_front_page_preview {
        format!("<a href='/posts/{}' class='unstyled-link'>", post.id)
    } else {
        "<span>".to_string()
    };
    let post_link_end = if is_front_page_preview {
        "</a>".to_string()
    } else {
        "</span>".to_string()
    };
    let post_preview_class = if is_front_page_preview {
        "post-preview"
    } else {
        ""
    };
    let slug = crate::md::extract_slug(post);
    let share_link = if is_front_page_preview {
        "".to_string()
    } else {
        let id = post.id;
        format!(
            "
            <div style='display: flex; justify-content: flex-end; \
              border-top: 1px solid var(--border); padding-top: 10px;
              font-size: var(--small-font-size);'>
                 <a href='/posts/{id}/{slug}' class='unstyled-link' id='long-url'>
                    🔗 Link
                 </a>&nbsp;(
                 <a id='copy-long-url' href='javascript:void(0)' onclick='copyLongUrl()'>
                    copy
                 </a>)
            </div>
            ",
        )
    };
    format!(
        "
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
            {share_link}
        </div>
        ",
        show_date(&post.created),
        post.id
    )
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
    /// Whether the user is logged in.
    ///
    /// None means don't show login/logout buttons.
    is_logged_in: Option<bool>,
    show_about: bool,
    top: Top,
    extra_head: String,
}

impl PageSettings {
    pub fn new(
        title: &str,
        is_logged_in: Option<bool>,
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
          oninput='disable_form_submit_if_empty(this);' \
          onchange='{SET_LEAVE_CONFIRMATION}' \
          id='content' name='content' placeholder='Your Markdown text..' required>
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
          oninput='disable_form_submit_if_empty(this);' \
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

async fn about(ctx: &ServerContext, settings: &PageSettings) -> String {
    let about = Kv::get(&*ctx.conn().await, "about").unwrap();
    let about = String::from_utf8(about).unwrap();
    let about = crate::md::content_to_html(&about);
    let author_name = Kv::get(&*ctx.conn().await, "author_name").unwrap();
    let author_name = String::from_utf8(author_name).unwrap();
    let style = "font-size: 0.8rem; padding-top: 0.1rem;";
    let admin_buttons = if settings.is_logged_in.unwrap_or(false) {
        &format!(
            "
            <span>
                <a href='/files' class='unstyled-link' style='{style}'>
                    📁 Files
                </a>&nbsp;
                <a href='/settings' class='unstyled-link' style='{style}'>
                    ⚙️ Settings
                </a>
            </span>
            "
        )
    } else {
        ""
    };
    let container_style = "display: flex; justify-content: space-between;";
    let name_style = "font-size: 1.2rem; margin-bottom: 10px; font-weight: bold;";
    let blogroll_key = crate::data::BLOGROLL_SETTINGS_KEY;
    let blogroll_feeds = match Kv::get(&*ctx.conn().await, blogroll_key) {
        Ok(feeds) => String::from_utf8(feeds).unwrap(),
        Err(_) => "".to_string(),
    };
    let blogroll_button = if blogroll_feeds.is_empty() {
        ""
    } else {
        &format!(
            "
            <a href='/blogroll' class='unstyled-link' style='{style}'>
                🔭 Blogroll
            </a>&nbsp;
            "
        )
    };
    format!(
        "
    <div class='introduction' style='padding: 10px; {}'>
        <div style='{container_style}'>
            <div class='full-name' \
                style='{name_style}'>
                <a class='unstyled-link' href='/'>{author_name}</a>
            </div>
            <div>
                <span>
                    <a href='/search' class='unstyled-link' style='{style}'>
                        🔍 Search
                    </a>&nbsp;
                    {blogroll_button}
                    <a href='/feed.xml' class='unstyled-link' style='{style}'>
                        🔄 RSS
                    </a>&nbsp;
                </span>
            </div>
        </div>
        <div class='about' style='font-size: 0.9rem;'>{about}</div>
        <div>
            {admin_buttons}
        </div>
    </div>
    ",
        border_style(2),
    )
}

fn katex_head(body: &str) -> String {
    let has_math = body.contains("<code class=\"language-math");
    let prefix = "https://cdn.jsdelivr.net/npm/katex@0.16.22/dist";
    if has_math {
        format!(
            "
            <link rel='stylesheet' href='{prefix}/katex.min.css' \
              crossorigin='anonymous'>
            <script defer src='{prefix}/katex.min.js' \
              crossorigin='anonymous'>
            </script>
            <script defer src='{prefix}/contrib/auto-render.min.js' \
              crossorigin='anonymous'>
            </script>
            <script defer src='/static/katex.js'>
            </script>
            "
        )
    } else {
        "".to_string()
    }
}

fn has_code(body: &str) -> bool {
    let re = r#"<code class="language-[^"]*""#;
    let rx = regex::Regex::new(re).unwrap();
    for cap in rx.captures_iter(body) {
        let (text, []) = cap.extract();
        if !text.contains("math") {
            return true;
        }
    }
    false
}

#[test]
fn test_has_code() {
    let body = indoc::indoc! {r#"
        <code class="language-math">
        x = 1
        </code>
        "#};
    assert!(!has_code(body));
    let body = indoc::indoc! {r#"
        <code class="language-rust">
        x = 1
        </code>
        "#};
    assert!(has_code(body));
}

fn contains_language(body: &str, language: &str) -> bool {
    let text = format!(r#"<code class="language-{language}""#);
    body.contains(&text)
}

fn highlight_head(body: &str) -> String {
    let prefix = "https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0";
    let julia = if contains_language(body, "julia") {
        format!(
            "
        <script src='{prefix}/languages/julia.min.js' defer></script>
        "
        )
    } else {
        "".to_string()
    };
    if has_code(body) {
        format!(
            "
            <link rel='stylesheet' href='{prefix}/styles/default.min.css' \
              media='(prefers-color-scheme: light)'>
            <link rel='stylesheet' href='{prefix}/styles/github-dark.min.css' \
              media='(prefers-color-scheme: dark)'>
            <script src='{prefix}/highlight.min.js' defer></script>
            {julia}
            <script defer>
                document.addEventListener('DOMContentLoaded', function() {{
                    document.querySelectorAll('pre code').forEach((el) => {{
                        if (el.classList.contains('language-math')) {{
                            return;
                        }}
                        hljs.highlightElement(el);
                    }});
                }});
            </script>
            "
        )
    } else {
        "".to_string()
    }
}

pub async fn page(ctx: &ServerContext, settings: &PageSettings, body: &str) -> String {
    let site_name = Kv::get(&*ctx.conn().await, "site_name").unwrap();
    let site_name = String::from_utf8(site_name).unwrap();
    let site_name = escape_single_quote(&site_name);
    let full_title = if settings.title.is_empty() {
        site_name.clone()
    } else {
        format!("{} - {site_name}", settings.title)
    };
    let about = if settings.show_about {
        about(ctx, settings).await
    } else {
        "".to_string()
    };
    let loginout = match settings.is_logged_in {
        Some(true) => r#"<a class="unstyled-link menu-space" href="/logout">Logout</a>"#,
        Some(false) => r#"<a class="unstyled-link menu-space" href="/login">Login</a>"#,
        None => "",
    };
    let top = match settings.top {
        Top::Homepage => {
            if settings.is_logged_in.unwrap_or(false) {
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
    let highlight = highlight_head(body);
    let katex = katex_head(body);
    let og_title = if settings.title.is_empty() {
        &site_name
    } else {
        &settings.title
    };
    let page = indoc::formatdoc! {
        r#"
        <!DOCTYPE html>
        <html lang='{html_lang}'>
        <head>
            <meta charset='UTF-8'>
            <meta name='viewport' content='width=device-width, initial-scale=1'>
            <link rel='stylesheet' href='/static/style.css'>
            <link rel='alternate' type='application/rss+xml' href='/feed.xml'>
            <script src='/static/script.js' defer></script>
            <title>{full_title}</title>
            <meta property='og:site_name' content='{site_name}'/>
            <meta property='og:title' content='{og_title}'/>
            {katex}
            {highlight}
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
            <script src='/static/nodefer.js'></script>
        </body>
        "#,
    };
    minify(&page)
}

pub async fn login(ctx: &ServerContext, error: Option<&str>) -> String {
    let top = Top::Homepage;
    let settings = PageSettings::new("login", None, false, top, "");
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
    page(ctx, &settings, &body).await
}
