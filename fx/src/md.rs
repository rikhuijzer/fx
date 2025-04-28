use crate::data::Post;
use markdown::Options;
use markdown::ParseOptions;
use markdown::mdast::Node;
use markdown::to_mdast;

pub fn markdown_link() -> &'static str {
    "<a href='https://www.markdownguide.org/'>Markdown</a>"
}

/// Convert a Markdown AST node back to a `String` with the same structure.
///
/// The default `to_string()` method only returns text.
fn node_to_html(node: &Node) -> String {
    let mut preview = String::new();
    match node {
        Node::Paragraph(paragraph) => {
            preview.push_str("<p>");
            for child in paragraph.children.iter() {
                let text = node_to_html(child);
                preview.push_str(&text);
            }
            preview.push_str("</p>");
        }
        Node::Heading(heading) => {
            preview.push_str(&format!("<h{}>", heading.depth));
            for child in heading.children.iter() {
                preview.push_str(&node_to_html(child));
            }
            preview.push_str(&format!("</h{}>", heading.depth));
            preview.push_str("\n\n");
        }
        Node::Text(text) => preview.push_str(&text.value),
        Node::Html(html) => preview.push_str(&html.value),
        Node::Link(link) => {
            let text = node_to_html(link.children.first().unwrap());
            let url = &link.url;
            preview.push_str(&format!("<a href='{url}'>{text}</a>"));
        }
        Node::List(list) => {
            let tag = if list.ordered { "ol" } else { "ul" };
            preview.push_str(&format!("<{tag}>"));
            for child in list.children.iter() {
                preview.push_str(&node_to_html(child));
            }
            preview.push_str(&format!("</{tag}>"));
        }
        Node::ListItem(list_item) => {
            preview.push_str("<li>");
            for child in list_item.children.iter() {
                preview.push_str(&node_to_html(child));
            }
            preview.push_str("</li>");
        }
        Node::Code(code) => {
            let lang = code.lang.clone().unwrap_or("".to_string());
            preview.push_str(&format!("\n\n```{lang}\n{}\n```\n", code.value));
        }
        Node::InlineCode(inline_code) => {
            preview.push_str(&format!("<code>{}</code>", inline_code.value));
        }
        _ => {}
    }
    preview
}

fn to_html_options() -> Options {
    let mut options = Options::default();
    options.compile.allow_dangerous_html = true;
    options
}

pub fn content_to_html(content: &str) -> String {
    let options = to_html_options();
    markdown::to_html_with_options(content, &options).unwrap()
}

/// Prepare post to be shown as preview.
pub fn sanitize_preview(post: &mut Post) {
    let options = ParseOptions::default();
    let tree = to_mdast(&post.content, &options).unwrap();
    let mut preview = String::new();
    let max_length = 600;
    for node in tree.children().unwrap() {
        if max_length < preview.len() {
            let id = post.id;
            let style = "text-decoration: underline; font-size: 0.8rem;";
            let expand = format!(
                "
                <p>
                    <a href='/posts/{id}' style='{style}'>
                        Show more
                    </a>
                </p>
                "
            );
            preview.push_str(&expand);
            break;
        }
        preview.push_str(&node_to_html(node));
    }
    post.content = preview;
}

#[test]
fn test_keep_link() {
    use chrono::Utc;
    let content = indoc::indoc! {"
        # Title

        Lorem ipsum [foo](https://example.com/foo) dolor sit amet
    "};
    let mut post = Post {
        id: 0,
        content: content.to_string(),
        created: Utc::now(),
        updated: Utc::now(),
    };
    sanitize_preview(&mut post);
    let expected = indoc::indoc! {"
        <h1>Title</h1>

        <p>Lorem ipsum <a href='https://example.com/foo'>foo</a> dolor sit amet</p>
    "};
    assert_eq!(post.content, expected.trim());
}

#[test]
fn test_sanitize_preview() {
    use chrono::Utc;
    // Need indoc to avoid indented lines to be interpreted as code.
    let content = indoc::indoc! {"
    Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed
    do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad
    minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex
    ea commodo consequat.
    
    Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod
    tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam,
    quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo
    consequat.
    
    Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod
    tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam,
    quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo
    consequat.
    
    Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod
    tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam,
    quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo
    consequat.
    "};
    let mut post = Post {
        id: 0,
        content: content.to_string(),
        created: Utc::now(),
        updated: Utc::now(),
    };
    sanitize_preview(&mut post);
    println!("post:\n{}", post.content);
    assert!(post.content.contains("Show more"));
    assert!(post.content.contains("<p>Lorem"));
}

fn remove_urls(md: &str) -> String {
    // This will break on nested links, but commonmark does not support nested
    // links according to <https://spec.commonmark.org/0.31.2/#links>.
    let re = regex::Regex::new(r"\[(.*?)\]\(https?://.*?\)").unwrap();
    re.replace_all(md, "$1").to_string()
}

fn truncate(text: &str, max_length: usize) -> String {
    let mut text = text.to_string();
    if text.len() > max_length {
        let mut pos = max_length;
        while pos > 0 && !text.is_char_boundary(pos) {
            pos -= 1;
        }
        text.truncate(pos);
    }
    text.trim().to_string()
}

pub fn extract_html_title(post: &Post) -> String {
    let title = &post.content;
    // This also would make a post with a single word on the first line have
    // that as the title which I guess makes sense.
    let title = title.split("\n").next().unwrap();
    let title = if title.starts_with("# ") {
        title.trim_start_matches("# ")
    } else {
        title
    };
    let title = remove_urls(title);
    // Better a bit too long than too short. Google truncates anyway.
    let max_length = 60;
    if title.len() <= max_length {
        title
    } else {
        format!("{}...", truncate(&title, max_length))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_extract_html_title() {
        let post = Post {
            id: 0,
            content: "[lorem](https://example.com/lorem) ipsum".to_string(),
            created: Utc::now(),
            updated: Utc::now(),
        };
        let title = extract_html_title(&post);
        assert_eq!(title, "lorem ipsum");

        let post = Post {
            id: 0,
            content: "# Title\nipsum".to_string(),
            created: Utc::now(),
            updated: Utc::now(),
        };
        let title = extract_html_title(&post);
        assert_eq!(title, "Title");
    }
}

/// Automatically turn URLs into CommonMark autolinks.
///
/// Turns `http://example.com` into `<http://example.com>`. Sites like Hacker
/// News and Lobsters do this too.
pub fn auto_autolink(content: &str) -> String {
    // Characters such as < are not allowed in URLs (should be percent encoded).
    let re = r#"<?https?://[^\s<>"{}|\\^`\(\)]+"#;
    let re = regex::Regex::new(re).unwrap();
    fn handle(caps: &regex::Captures) -> String {
        let url = caps.get(0).unwrap().as_str();
        // Cannot use look-around, so manually avoiding double wrapping autolink
        // inside autolink.
        if url.starts_with("<") {
            url.to_string()
        } else {
            format!("<{url}>")
        }
    }
    re.replace_all(content, handle).to_string()
}

#[test]
fn test_auto_autolink() {
    let content = "Foo http://example.com bar";
    let expected = "Foo <http://example.com> bar";
    let actual = auto_autolink(content);
    assert_eq!(actual, expected);

    let content = "https://example.com";
    let expected = "<https://example.com>";
    let actual = auto_autolink(content);
    assert_eq!(actual, expected);

    let content = "<p>Lorem https://example.com</p>";
    let expected = "<p>Lorem <https://example.com></p>";
    let actual = auto_autolink(content);
    assert_eq!(actual, expected);

    let content = "<p>Lorem (https://example.com)</p>";
    let expected = "<p>Lorem (<https://example.com>)</p>";
    let actual = auto_autolink(content);
    assert_eq!(actual, expected);

    let content = "<p>Lorem <https://example.com></p>";
    let expected = "<p>Lorem <https://example.com></p>";
    let actual = auto_autolink(content);
    assert_eq!(actual, expected);
}
