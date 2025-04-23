use crate::data::Post;
use markdown::Options;
use markdown::ParseOptions;
use markdown::mdast::Node;
use markdown::to_mdast;

fn without_links_core(node: &Node) -> String {
    let mut preview = String::new();
    match node {
        Node::Paragraph(paragraph) => {
            for child in paragraph.children.iter() {
                preview.push_str(&without_links_core(child));
            }
        }
        Node::Heading(heading) => {
            preview.push_str(&"#".repeat(heading.depth as usize));
            preview.push(' ');
            for child in heading.children.iter() {
                preview.push_str(&without_links_core(child));
            }
            preview.push_str("\n\n");
        }
        Node::Text(text) => preview.push_str(&text.value),
        Node::Html(html) => preview.push_str(&html.value),
        Node::Link(link) => {
            let text = without_links_core(link.children.first().unwrap());
            let url = &link.url;
            preview.push_str(&format!("<a href='{url}'>{text}</a>"));
        }
        Node::Code(code) => {
            let lang = code.lang.clone().unwrap_or("".to_string());
            preview.push_str(&format!("\n\n```{}\n{}\n```\n", lang, code.value));
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

pub fn to_html(content: &str) -> String {
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
            // Not adding a link because a preview is already a link.
            let expand = indoc::indoc! {"
                \\
                <span style='text-decoration: underline; font-size: 0.8rem;'>
                    Show more
                </span>
            "};
            preview.push_str(expand);
            break;
        }
        preview.push_str(&without_links_core(node));
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
        # Title

        Lorem ipsum <a href='https://example.com/foo'>foo</a> dolor sit amet
    "};
    assert_eq!(post.content, expected.trim());
}

#[test]
fn test_truncate() {
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
    println!("post: {}", post.content);
    assert!(post.content.contains("Show more"));
}
