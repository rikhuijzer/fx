pub fn page(body: &str) -> String {
    let html = indoc::formatdoc! {
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta charset="UTF-8">
            <meta name="viewport" content="width=device-width, initial-scale=1">
            <link rel="stylesheet" href="/static/style.css">
            <title>fedx</title>
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
