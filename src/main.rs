use clap::Parser;
use std::{process::Command, time::Duration};

use anyhow::Context;

mod commit;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The command to run on remote branch changes
    #[arg(last = true, required = true)]
    cmd: Vec<String>,
}

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
    // Parse the arguments
    let args = Args::parse();
    let user_cmd: String = args.cmd.join(" ");

    // Retrieve the token
    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .map_err(|_| {
            anyhow::anyhow!("You must set the GH_TOKEN or GITHUB_TOKEN environment variable")
        })?;

    // TODO(alvaro): We can detect the current checked out branch
    // We can also detect the user and repo from the git config
    // But we should also allow for overriding this with flags or env
    // variables
    let repo = RemoteRepo::try_from_gitconfig()?;

    println!("Found RemoteRepo information: {:#?}", &repo);

    let mut state = GithubState::new(
        repo.username.clone(),
        repo.repo_name.clone(),
        repo.branch.clone(),
        token.to_string(),
    );

    let seconds = 5;

    // Prepare the command associated to the user
    let cmd_parts =
        shellish_parse::parse(&user_cmd, true).context("could not parse user command")?;
    let cmd_name = &cmd_parts[0];
    let cmd_args = &cmd_parts[1..];

    // Refresh the state every N seconds
    loop {
        let previous = state.refresh()?;
        println!("The last commit is: {:?}", state.last_commit());
        if previous.as_deref() != state.last_commit() {
            // Actually run the command
            println!("IT CHANGED!");
            Command::new("git")
                .arg("pull")
                .arg("--ff-only")
                .status()
                .context("failed to execute git pull")?
                .code()
                .map(|_| println!("Pulled the latest changes"))
                .ok_or(anyhow::anyhow!("pull returned error"))?;

            // Run here the user command
            Command::new(cmd_name)
                .args(cmd_args)
                .status()
                .context("failed to execute user command")?
                .code()
                .ok_or(anyhow::anyhow!("user command returned error"))?;
        }

        // Sleep for some time
        std::thread::sleep(Duration::from_secs(seconds));
    }
}

/// The information about the remote repository
#[derive(Debug)]
struct RemoteRepo {
    /// The name of the account that owns the repository
    pub username: String,
    /// The name of the repository
    pub repo_name: String,
    /// The name of the branch to track
    pub branch: String,
}

impl RemoteRepo {
    /// Initialize a RemoteRepo based on the given values or by guessing from the
    /// git configuration
    fn try_from_gitconfig() -> anyhow::Result<Self> {
        // Extract the branch name
        let branch = git_head()?;
        // Extract the information from the upstream remote
        let (username, repo_name) = git_upstream_info(&branch)?;

        Ok(Self {
            username,
            repo_name,
            branch,
        })
    }
}

fn git_head() -> anyhow::Result<String> {
    // TODO(alvaro): Make it work with an arbitrary branch
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .context("failed to execute git rev-parse HEAD")?;

    if !output.status.success() {
        anyhow::bail!(
            "failed to get upstream branch (code={}): {}",
            &output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get the remote username and repo_name from the git remote information
fn git_upstream_info(_branch: &str) -> anyhow::Result<(String, String)> {
    // TODO(alvaro): Make it work with an arbitrary branch
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("--symbolic-full-name")
        .arg("@{upstream}")
        .output()
        .context("failed to execute git rev-parse <branch>")?;

    if !output.status.success() {
        anyhow::bail!(
            "failed to get upstream branch (code={}): {}",
            &output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let upstream = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let remote = upstream.rsplit('/').nth(1).ok_or(anyhow::anyhow!(
        "could not get remote name from upstream branch: {}",
        upstream
    ))?;

    // TODO(alvaro): Make it work with an arbitrary branch
    // Get the information from the remote
    let output = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg(remote)
        .output()
        .context("failed to execute git remote get-url <remote>")?;

    if !output.status.success() {
        anyhow::bail!(
            "failed to get remote url (code={}): {}",
            &output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Parse the URL
    let (username, repo_name) = if url.starts_with("git@") {
        // It's an SSH URL
        let repo_uri = url.rsplit(':').next().ok_or(anyhow::anyhow!(
            "failed to get the repo_uri from the remote url: {}",
            url
        ))?;
        repo_uri
            .split_once('/')
            .expect("the repo uri to have a slash")
    } else {
        // It's an HTTP(s) URL
        assert!(url.starts_with("http"));

        let mut uri_parts = url.split('/');
        let repo_name = uri_parts
            .next()
            .expect("split to return at least one result");
        let username = uri_parts.next().ok_or(anyhow::anyhow!(
            "the URI to have at least one slash: {}",
            url
        ))?;

        (username, repo_name)
    };

    // Trim the `.git` suffix, it it's there
    let repo_name = if repo_name.ends_with(".git") {
        repo_name.strip_suffix(".git").unwrap()
    } else {
        repo_name
    };

    Ok((username.to_string(), repo_name.to_string()))
}
