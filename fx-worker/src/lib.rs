use worker::*;

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let router = Router::new()
        .get_async("/", |_, _ctx| async move { Response::ok("Hello") })
        .get_async("/test", |_, ctx| async move {
            let db = ctx.env.d1("FXDB")?;
            let stmt =
                db.prepare("CREATE TABLE IF NOT EXISTS kv (key TEXT PRIMARY KEY, value BLOB)");
            let query = stmt.bind(&[]).unwrap();
            query.run().await.unwrap();
            Response::ok("added table")
        });

    router.run(req, env).await
}
