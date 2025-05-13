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
                &format!("class='language-{lang}'")
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
        Node::FootnoteDefinition(_footnote_definition) => {},
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

pub fn extract_html_description(post: &Post) -> String {
    let content = post.content.trim();
    let title = extract_html_title(post);
    let title = title.trim_end_matches("...");
    let description = remove_urls(content);
    let description = description.trim_start_matches("# ");
    let description = description.trim_start_matches(title).trim();
    let description = remove_urls(description);
    let max_length = 150;
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
