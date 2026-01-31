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
    // Maybe this method should be rewritten to return Markdown or use some
    // internal logic from the `markdown` crate for the preview. I think the
    // reason that this part is converting to HTML now is that HTML is more
    // clear than Markdown. The benefit of having this code here is to later be
    // able to easily modify the Markdown node to HTML conversion. For example,
    // to remove hyperlinks from some Markdown code.
    let mut preview = String::new();
    match node {
        Node::Code(code) => {
            let lang = code.lang.clone().unwrap_or("".to_string());
            let class = if lang.is_empty() {
                ""
            } else {
                &format!("class=\"language-{lang}\"")
            };
            let html = format!(
                "
                <pre><code {class}>{}</code></pre>
                ",
                code.value
            );
            preview.push_str(&html);
        }
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
        Node::Emphasis(emphasis) => {
            preview.push_str("<em>");
            for child in emphasis.children.iter() {
                preview.push_str(&node_to_html(child));
            }
            preview.push_str("</em>");
        }
        Node::Strong(strong) => {
            preview.push_str("<strong>");
            for child in strong.children.iter() {
                preview.push_str(&node_to_html(child));
            }
            preview.push_str("</strong>");
        }
        Node::Delete(delete) => {
            preview.push_str("<del>");
            for child in delete.children.iter() {
                preview.push_str(&node_to_html(child));
            }
            preview.push_str("</del>");
        }
        Node::Text(text) => preview.push_str(&text.value),
        Node::Html(html) => preview.push_str(&html.value),
        Node::Link(link) => {
            let text = node_to_html(link.children.first().unwrap());
            let url = &link.url;
            preview.push_str(&format!("<a href='{url}'>{text}</a>"));
        }
        Node::Math(math) => {
            preview.push_str(&format!(
                r#"
                <pre><code class="language-math math-display">{}
                </code></pre>
                "#,
                math.value
            ));
        }
        Node::Table(table) => {
            preview.push_str("<table>");
            for child in table.children.iter() {
                preview.push_str(&node_to_html(child));
            }
            preview.push_str("</table>");
        }
        Node::TableRow(table_row) => {
            preview.push_str("<tr>");
            for child in table_row.children.iter() {
                preview.push_str(&node_to_html(child));
            }
            preview.push_str("</tr>");
        }
        Node::Break(_br) => {
            preview.push_str("<br />");
        }
        Node::TableCell(table_cell) => {
            preview.push_str("<td>");
            for child in table_cell.children.iter() {
                preview.push_str(&node_to_html(child));
            }
            preview.push_str("</td>");
        }
        Node::InlineCode(inline_code) => {
            preview.push_str(&format!("<code>{}</code>", inline_code.value));
        }
        Node::InlineMath(inline_math) => {
            let value = &inline_math.value;
            preview.push_str(&format!("<code class=\"language-math\">{value}</code>"));
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
        Node::Image(image) => {
            let url = &image.url;
            let alt = &image.alt;
            preview.push_str(&format!("<img src='{url}' alt='{alt}' />"));
        }
        Node::FootnoteDefinition(_footnote_definition) => {}
        Node::Blockquote(blockquote) => {
            preview.push_str("<blockquote>");
            for child in blockquote.children.iter() {
                preview.push_str(&node_to_html(child));
            }
            preview.push_str("</blockquote>");
        }
        Node::FootnoteReference(_footnote_reference) => {}
        _ => {}
    }
    preview
}

fn parse_options() -> ParseOptions {
    let mut options = ParseOptions::default();
    options.constructs.gfm_table = true;
    // KaTeX does not exactly match CommonMark, so we parse math to HTML and
    // then figure out how to render the produced code blocks in Javascript.
    // Note that this also makes it easy to detect whether a post contains math,
    // and thus easy to decide whether to load KaTeX.
    options.constructs.math_flow = true;
    options.constructs.math_text = true;
    options.constructs.gfm_footnote_definition = true;
    options.constructs.gfm_label_start_footnote = true;
    options
}

fn to_html_options() -> Options {
    let mut options = Options::default();
    options.compile.allow_dangerous_html = true;
    options.parse = parse_options();
    options
}

pub fn content_to_html(content: &str) -> String {
    let options = to_html_options();
    markdown::to_html_with_options(content, &options).unwrap()
}

/// Prepare post to be shown as preview.
pub fn preview(post: &mut Post, max_length: usize) {
    let options = parse_options();
    let tree = to_mdast(&post.content, &options).unwrap();
    let mut preview = String::new();
    let slug = crate::md::extract_slug(post);
    for node in tree.children().unwrap() {
        if max_length < preview.len() {
            let id = post.id;
            let style = "font-size: 0.8rem;";
            let expand = format!(
                "
                <p>
                    <a href='/posts/{id}/{slug}' style='{style}'>
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
    preview(&mut post, 600);
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
    preview(&mut post, 600);
    println!("post:\n{}", post.content);
    assert!(post.content.contains("Show more"));
    assert!(post.content.contains("<p>Lorem"));
}

fn remove_markdown_links(md: &str) -> String {
    // This will break on nested links, but commonmark does not support nested
    // links according to <https://spec.commonmark.org/0.31.2/#links>.
    //
    // First, remove image markdown entirely (no useful text to keep).
    // Matches: ![alt](url) where url can be any content (not just http/https).
    let re_img = regex::Regex::new(r"!\[[^\]]*\]\([^)]*\)").unwrap();
    let md = re_img.replace_all(md, "");
    // Then, remove link markdown but keep the link text.
    // Matches: [text](url) where url can be any content.
    let re_link = regex::Regex::new(r"\[([^\]]*)\]\([^)]*\)").unwrap();
    re_link.replace_all(&md, "$1").to_string()
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
    // Remove trailing newlines.
    let title = title.trim();
    let title = remove_markdown_links(title);
    // Better a bit too long than too short. Google truncates anyway.
    let max_length = 60;
    if title.len() <= max_length {
        title
    } else {
        format!("{}...", truncate(&title, max_length))
    }
}

/// Extract a slug (a short URL suffix to clarify the post) from the post.
///
/// For example, a post with the title `Foo Bar` and id `1` would receive the
/// slug `foo-bar` so that the post can be shared as `/posts/1/foo-bar`.
///
/// This function should be called before `crate::md::preview` because otherwise
/// the content is converted to Markdown and texts such as `<p><a href` will end
/// up in the slug
pub fn extract_slug(post: &Post) -> String {
    let title = extract_html_title(post);
    let slug = title.replace(" ", "-");
    let slug = slug
        .replace(",", "")
        .replace("\"", "")
        .replace("'", "")
        .replace(":", "")
        .replace(";", "")
        .replace("!", "")
        .replace("?", "")
        .replace(".", "")
        // Forward slashes break URL routing by adding extra path segments.
        .replace("/", "")
        // Brackets and parentheses from markdown image/link syntax.
        .replace("[", "")
        .replace("]", "")
        .replace("(", "")
        .replace(")", "")
        // Other URL-unsafe characters.
        .replace("#", "")
        .replace("&", "")
        .replace("%", "")
        .replace("@", "")
        .replace("*", "")
        .replace("~", "")
        .replace("\\", "")
        .replace("`", "")
        .replace("^", "")
        .replace("|", "")
        .replace("<", "")
        .replace(">", "")
        .replace("{", "")
        .replace("}", "")
        .replace("=", "")
        .replace("+", "")
        .to_lowercase();
    let max_length = 50;
    if slug.len() <= max_length {
        slug
    } else {
        truncate(&slug, max_length)
    }
}

#[test]
fn test_extract_slug() {
    let mut post = Post {
        id: 0,
        content: "Foo Bar".to_string(),
        created: chrono::Utc::now(),
        updated: chrono::Utc::now(),
    };
    assert_eq!(extract_slug(&post), "foo-bar");
    post.content = "Lorem, ipsum".to_string();
    assert_eq!(extract_slug(&post), "lorem-ipsum");

    // Forward slash in title should be removed to avoid breaking URL routing.
    post.content = "C++/Rust comparison".to_string();
    assert_eq!(extract_slug(&post), "c++rust-comparison");

    post.content = "How to use / operator".to_string();
    assert_eq!(extract_slug(&post), "how-to-use--operator");

    // Image markdown as first line should result in empty slug (image removed).
    post.content = "![](https://example.com/image.png)".to_string();
    assert_eq!(extract_slug(&post), "");

    // Image with protocol-less URL should also be removed.
    post.content = "![](imageurl.tld)".to_string();
    assert_eq!(extract_slug(&post), "");

    // Image followed by text should keep only the text.
    post.content = "![](image.png) Hello World".to_string();
    assert_eq!(extract_slug(&post), "hello-world");

    // Link with protocol-less URL should keep link text.
    post.content = "[click here](page.html)".to_string();
    assert_eq!(extract_slug(&post), "click-here");

    // Brackets and parentheses should be removed as safety net.
    post.content = "[important] news (breaking)".to_string();
    assert_eq!(extract_slug(&post), "important-news-breaking");
}

pub fn extract_html_description(post: &Post) -> String {
    let content = post.content.trim();
    let title = extract_html_title(post);
    let title = title.trim_end_matches("...");
    let description = remove_markdown_links(content);
    let description = description.trim_start_matches("# ");
    let description = description.trim_start_matches(title).trim();
    let description = remove_markdown_links(description);
    // This allows RSS readers to read the full quote when the page is a
    // microblog and still truncates to avoid having a too heavy RSS feed.
    let max_length = 600;
    if description.len() <= max_length {
        description
    } else {
        format!("{}...", truncate(&description, max_length))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_extract_html_title_and_description() {
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
        let description = extract_html_description(&post);
        assert_eq!(description, "ipsum");
    }
}
