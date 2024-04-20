use std::{process::Command, time::Duration};

use anyhow::Context;

mod commit;

#[derive(Debug)]
struct GithubState {
    username: String,
    repo: String,
    branch: String,
    token: String,
    client: reqwest::blocking::Client,
    last_commit: Option<String>,
}

impl GithubState {
    fn new(username: String, repo: String, branch: String, token: String) -> Self {
        // NOTE(alvaro): Github apparently is blocking based on user agent (maybe
        // the problem is a missing user agent?)
        let curl_ua = "curl/7.68.0";
        let client = reqwest::blocking::Client::builder()
            .user_agent(curl_ua)
            .build()
            .expect("the client to build");

        Self {
            username,
            repo,
            branch,
            token,
            client,
            last_commit: None,
        }
    }

    pub fn last_commit(&self) -> Option<&str> {
        self.last_commit.as_deref()
    }

    fn refresh(&mut self) -> anyhow::Result<Option<String>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/commits?sha={}&per_page=1",
            self.username, self.repo, self.branch
        );
        let body = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()?
            .error_for_status()?;

        let commits: Vec<commit::CommitResponse> = body.json()?;
        let commit = commits
            .into_iter()
            .next()
            .ok_or(anyhow::anyhow!("No commits found"))?;
        Ok(self.last_commit.replace(commit.sha))
    }
}

fn main() -> anyhow::Result<()> {
    // Retrieve the token
    let token = std::env::var("GITHUB_TOKEN")
        .context("You must set the GITHUB_TOKEN environment variable")?;

    let username = "alvaroclementev";
    let repo = "test-repo";
    let branch = "new";

    let mut state = GithubState::new(
        username.to_string(),
        repo.to_string(),
        branch.to_string(),
        token.to_string(),
    );

    let seconds = 5;

    // Refresh the state every N seconds
    loop {
        let previous = state.refresh()?;
        println!("The last commit is: {:?}", state.last_commit());
        if previous.as_deref() != state.last_commit() {
            // Actually run the command
            println!("IT CHANGED!");
            let status = Command::new("git")
                .arg("pull")
                .arg("--ff-only")
                .status()
                .expect("failed to execute git pull");

            status
                .code()
                .map(|_| println!("Pulled the latest changes"))
                .ok_or(anyhow::anyhow!("failed to pull"))?;

            // TODO(alvaro): Run here the specified command
        }

        // Sleep for some time
        std::thread::sleep(Duration::from_secs(seconds));
    }
}
