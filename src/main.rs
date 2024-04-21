use clap::Parser;
use std::{process::Command, time::Duration};

use anyhow::Context;

mod commit;
mod git;

/// Listen for changes on the upstream for the currently checked out branch,
/// and when a change is found, pull them and run the given command
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The polling period for checking for upstream changes (in seconds)
    #[arg(default_value = "5", long, short)]
    period: f64,

    /// The command to run on upstream changes
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

    // Prepare the command associated to the user
    let cmd_parts =
        shellish_parse::parse(&user_cmd, true).context("could not parse user command")?;
    let cmd_name = &cmd_parts[0];
    let cmd_args = &cmd_parts[1..];

    let mut state = setup()?;

    println!(
        "Listening for changes from {}/{}/{}",
        &state.username, &state.repo, &state.branch
    );

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
        std::thread::sleep(Duration::from_secs_f64(args.period));
    }
}

/// Analyze the executing environment and collect the state
fn setup() -> anyhow::Result<GithubState> {
    // Retrieve the token
    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .map_err(|_| {
            anyhow::anyhow!("You must set the GH_TOKEN or GITHUB_TOKEN environment variable")
        })?;

    // Check if there are in a git repository work tree
    if !git::git_is_work_tree()? {
        anyhow::bail!("you are not in a git repository");
    }

    // TODO(alvaro): We can detect the current checked out branch
    // We can also detect the user and repo from the git config
    // But we should also allow for overriding this with flags or env
    // variables
    let repo = RemoteRepo::try_from_gitconfig()?;

    // Check if there are some unstashed changes
    if git::git_has_unstashed_changes()? {
        anyhow::bail!("there are uncommitted changes. Run `git commit` or `git stash` to save the changes, and try again.");
    }

    Ok(GithubState::new(
        repo.username.clone(),
        repo.repo_name.clone(),
        repo.branch.clone(),
        token.to_string(),
    ))
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
        let branch = git::git_head()?;
        // Extract the information from the upstream remote
        let (username, repo_name) = git::git_upstream_info(&branch)?;

        Ok(Self {
            username,
            repo_name,
            branch,
        })
    }
}
