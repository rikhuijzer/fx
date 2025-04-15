use crate::data::Post;

pub trait ToHtml {
    fn to_html(&self) -> String;
}

impl ToHtml for Post {
    fn to_html(&self) -> String {
        indoc::formatdoc! {"
        <div class='post' hx-boost='true'>
            <div class='created_at'>{}</div>
            <a style='text-decoration: none; color: inherit;' href='/p/{}'>
                <div class='content'>{}</div>
            </a>
        </div>
        ", self.created_at, self.id, self.content}
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

pub fn page(body: &str) -> String {
    let htmx = htmx();
    let html = indoc::formatdoc! {
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <link rel="stylesheet" href="/static/style.css">
            <title>fx</title>
            {htmx}
        </head>
        <body>
            <div class="container">
                <div class="middle">
                    {body}
                </div>
            </div>
        </body>
        "#
    };
    html
}
