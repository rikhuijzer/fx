//! Trigger GitHub Actions.
use crate::serve::ServerContext;
use hyper::HeaderMap;
use hyper::header;
use hyper::header::HeaderValue;

struct TriggerArgs {
    pub trigger_token: Option<String>,
    pub trigger_owner_repo: Option<String>,
    pub trigger_branch: String,
    pub trigger_workflow_id: String,
}

async fn trigger_github_backup_workload(args: TriggerArgs) -> Option<()> {
    let token = match &args.trigger_token {
        Some(token) => token,
        None => return None,
    };
    let owner_repo = match &args.trigger_owner_repo {
        Some(owner_repo) => owner_repo,
        None => return None,
    };
    let branch = &args.trigger_branch;
    let workflow = &args.trigger_workflow_id;

    let domain = "https://api.github.com";
    let url = format!("{domain}/repos/{owner_repo}/actions/workflows/{workflow}/dispatches",);
    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        header::ACCEPT,
        "application/vnd.github.v3+json".parse().unwrap(),
    );
    headers.insert(
        header::AUTHORIZATION,
        format!("Bearer {}", token).parse().unwrap(),
    );
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );
    headers.insert(
        header::USER_AGENT,
        format!("fx/{}", env!("CARGO_PKG_VERSION")).parse().unwrap(),
    );
    let body = format!(r#"{{"ref":"{branch}"}}"#);
    let response = client.post(url).headers(headers).body(body).send().await;
    match response {
        Ok(response) => {
            if response.status().is_success() {
                Some(())
            } else {
                tracing::error!(
                    "Failed to trigger GitHub Actions: {}",
                    response.text().await.unwrap()
                );
                None
            }
        }
        Err(_) => None,
    }
}

pub async fn trigger_github_backup(ctx: &ServerContext) -> Option<()> {
    let args = TriggerArgs {
        trigger_token: ctx.args.trigger_token.clone(),
        trigger_owner_repo: ctx.args.trigger_owner_repo.clone(),
        trigger_branch: ctx.args.trigger_branch.clone(),
        trigger_workflow_id: ctx.args.trigger_workflow_id.clone(),
    };
    // Based on the docs, `tokio::spawn` will start running the task even when
    // not awaiting the future.  However, it also states that the task will not
    // be executed to completion if the runtime is shutdown.
    tokio::spawn(async { trigger_github_backup_workload(args) });
    Some(())
}
