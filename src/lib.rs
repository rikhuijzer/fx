use sqlx_d1::query;

#[worker::event(fetch)]
async fn main(
    mut req: worker::Request,
    env: worker::Env,
    _ctx: worker::Context,
) -> worker::Result<worker::Response> {
    let d1 = env.d1("DB")?;
    let conn = sqlx_d1::D1Connection::new(d1);

    #[derive(serde::Deserialize)]
    struct CreatePost {
        date: chrono::DateTime<chrono::Utc>,
        content: String,
    }

    let req = req.json::<CreatePost>().await?;

    // ```
    // wrangler d1 migrations apply DB --local
    // ```

    let id = query!(
        "
        INSERT INTO posts (created_at, content) VALUES (?, ?)
        RETURNING id
        ",
        chrono::Utc::now(),
        req.content
    )
    .fetch_one(&conn)
        .await
        .map_err(|e| worker::Error::RustError(e.to_string()))?
        .id;

    worker::Response::ok(format!("Your id is {id}!"))
}
