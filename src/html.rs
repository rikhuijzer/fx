use crate::data::Post;

pub trait ToHtml {
    fn to_html(&self) -> String;
}

impl ToHtml for Post {
    fn to_html(&self) -> String {
        indoc::formatdoc! {"
        <div class='post' hx-boost='true'>
            <div class='created_at'>{}</div>
            <a class='unstyled-link' href='/p/{}'>
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

pub fn page(title: &str, body: &str) -> String {
    let htmx = htmx();
    let title = if title.is_empty() {
        "fx".to_string()
    } else {
        format!("{title} - fx")
    };
    let html = indoc::formatdoc! {
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <link rel="stylesheet" href="/static/style.css">
            <title>{title}</title>
            {htmx}
        </head>
        <body>
            <div class="container">
                <div class="middle">
                    <div class="top">
                        <a class="unstyled-link" href="/">fx</a>
                    </div>
                    {body}
                    <div class="bottom">
                        <a class="unstyled-link" href="https://github.com/rikhuijzer/fx">source</a>
                    </div>
                </div>
            </div>
        </body>
        "#
    };
    html
}

pub fn login() -> String {
    page(
        "login",
        r#"
    <form style="text-align: center;" method="post" action="/login">
        <label for="username">username</label><br>
        <input id="username" name="username" type="text" required/><br>
        <br>
        <label for="password">password</label><br>
        <input id="password" name="password" type="password" required/><br>
        <br>
        <input type="submit" value="login"/>
    </form>
    "#,
    )
}
