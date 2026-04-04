use tracing::Level;
use tracing::subscriber::SetGlobalDefaultError;

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

pub fn now() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S%.3f UTC")
        .to_string()
}
