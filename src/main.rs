use axum::Router;
use axum::routing::get;

#[derive(Debug, clap::Subcommand)]
enum Task {
    /// Start the server.
    Serve,
    /// Print the project's license.
    License,
}

#[derive(Debug, clap::Parser)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    task: Task,
}

async fn home() -> &'static str {
    "Hello, World!"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let app = Router::new().route("/", get(home));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
