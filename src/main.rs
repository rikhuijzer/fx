mod data;
mod serve;

use clap::Parser;

#[derive(Debug, clap::Subcommand)]
enum Task {
    /// Start the server.
    Serve,
    /// Print the project's license.
    License,
}

#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(long)]
    production: bool,
    #[command(subcommand)]
    task: Task,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();

    match args.task {
        Task::Serve => {
            serve::run(args.production);
        }
        Task::License => {
            let license_content = include_str!("../LICENSE");
            println!("{}", license_content);
        }
    }
}
