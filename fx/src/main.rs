use clap::Parser;
use fx::ServeArgs;
use fx::health::HealthArgs;
use tracing::Level;
use tracing::subscriber::SetGlobalDefaultError;

#[derive(Debug, clap::Subcommand)]
#[allow(clippy::large_enum_variant)]
enum Task {
    /// Run a health check on the given port.
    CheckHealth(HealthArgs),
    /// Print the project's license.
    License,
    /// Start the server.
    Serve(ServeArgs),
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

#[tokio::main]
async fn main() {
    let args = Args::parse();
    init_subscriber(Level::INFO, args.ansi.unwrap_or(true)).unwrap();

    match &args.task {
        Task::CheckHealth(args) => {
            fx::health::check_health(args).await;
        }
        Task::License => {
            let license_content = include_str!("../../LICENSE");
            println!("{}", license_content);
        }
        Task::Serve(args) => {
            fx::serve::run(args).await;
        }
    }
}
