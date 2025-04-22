use clap::Parser;
use fx::ServeArgs;
use tracing::Level;
use tracing::subscriber::SetGlobalDefaultError;

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Task {
    /// Start the server.
    Serve(ServeArgs),
    /// Print the project's license.
    License,
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// Whether to print verbose logs.
    #[arg(long)]
    verbose: bool,
    /// Whether to use ANSI colors.
    #[arg(long)]
    ansi: Option<bool>,
    #[command(subcommand)]
    task: Task,
}

/// Initialize logging with the given level.
pub fn init_subscriber(level: Level, ansi: bool) -> Result<(), SetGlobalDefaultError> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(level)
        .with_test_writer()
        .without_time()
        .with_target(false)
        .with_ansi(ansi)
        // Write logs to stderr to allow writing sha output to stdout.
        .with_writer(std::io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
}

async fn serve(args: &ServeArgs) {
    let conn = fx::data::connect(args).unwrap();
    fx::data::init(args, &conn);
    let salt = fx::serve::obtain_salt(args, &conn);
    let ctx = fx::serve::ServerContext::new(args.clone(), conn, salt);
    let app = fx::serve::app(ctx);
    let addr = format!("0.0.0.0:{}", args.port);
    tracing::info!("Listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    init_subscriber(Level::INFO, args.ansi.unwrap_or(true)).unwrap();

    match &args.task {
        Task::Serve(args) => {
            serve(args).await;
        }
        Task::License => {
            let license_content = include_str!("../../LICENSE");
            println!("{}", license_content);
        }
    }
}
