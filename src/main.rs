mod data;
mod serve;

use clap::Parser;

#[derive(Debug, Parser)]
pub struct ServeArgs {
    #[arg(long, env = "PRODUCTION")]
    production: bool,
    #[arg(long, env = "PORT", default_value = "3000")]
    port: u16,
    #[arg(long, env = "DATABASE_PATH", default_value = "/data/db.sqlite")]
    database_path: String,
}

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
            serve::run(args).await;
        }
        Task::License => {
            let license_content = include_str!("../LICENSE");
            println!("{}", license_content);
        }
    }
}
