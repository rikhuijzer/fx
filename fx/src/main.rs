use clap::Parser;
use fx::ServeArgs;
use fx::health::HealthArgs;
use fx::log::init_subscriber;
use tracing_core::Level;

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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match &args.task {
        Task::CheckHealth(args) => {
            fx::health::check_health(args).await;
        }
        Task::License => {
            let license_content = include_str!("../../LICENSE");
            println!("{}", license_content);
        }
        Task::Serve(serve_args) => {
            let log_level = match serve_args.log_level.as_str() {
                "error" => Level::ERROR,
                "warn" => Level::WARN,
                "info" => Level::INFO,
                "debug" => Level::DEBUG,
                _ => Level::INFO,
            };
            init_subscriber(log_level, args.ansi.unwrap_or(true)).unwrap();
            fx::serve::run(serve_args).await;
        }
    }
}
