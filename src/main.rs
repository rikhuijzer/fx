use clap::Parser;
use fedx::ServeArgs;

#[derive(Debug, clap::Subcommand)]
enum Task {
    /// Start the server.
    Serve(ServeArgs),
    /// Print the project's license.
    License,
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[command(subcommand)]
    task: Task,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match &args.task {
        Task::Serve(args) => {
            fedx::serve::run(args).await;
        }
        Task::License => {
            let license_content = include_str!("../LICENSE");
            println!("{}", license_content);
        }
    }
}
