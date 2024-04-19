use anyhow::Context;

mod commit;

fn main() -> anyhow::Result<()> {
    // Load the environment
    let branch = "new";

    // NOTE(alvaro): Github apparently is blocking based on user agent (maybe
    // the problem is a missing user agent?)
    let curl_ua = "curl/7.68.0";
    let client = reqwest::blocking::Client::builder()
        .user_agent(curl_ua)
        .build()?;

    let token = std::env::var("GITHUB_TOKEN")
        .context("You must set the GITHUB_TOKEN environment variable")?;
    let url = format!(
        "https://api.github.com/repos/alvaroclementev/test-repo/commits?sha={branch}&per_page=5"
    );

    let body = client
        .get(url)
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()?
        .error_for_status()?;

    let commit: Vec<commit::CommitResponse> = body.json()?;
    println!("{:#?}", commit);

    Ok(())
}
