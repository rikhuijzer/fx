use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct HealthArgs {
    #[arg(long, env = "FX_PORT", default_value = "3000")]
    pub port: u16,
}

/// Check the health of the fx service running at `args.port`.
///
/// This function essentially allows testing the service in environments where
/// `curl` or `wget` is not available. It exists with an error code when the
/// server does not respond or provides an invalid response.
///
/// Note to developer: this simple function can be tested by starting the server
/// and running `cargo run -- check-health`.
pub async fn check_health(args: &HealthArgs) {
    let port = args.port;
    // Requesting the main page since this is the most important page of the
    // site.
    let url = format!("http://localhost:{port}");
    let body = reqwest::get(url)
        .await
        .expect("Server did not respond")
        .text()
        .await
        .expect("Could not convert the response to text");
    if !body.contains("<!DOCTYPE html>") {
        eprint!("Expected to receive valid HTML, but got:\n{body}");
        std::process::exit(1);
    }
    println!("Successfully received response from server");
}
