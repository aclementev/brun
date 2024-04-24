mod commit;
mod error;
mod git;

use log::{debug, trace};
use std::io::{self, Write};
use std::{process::Command, time::Duration};

use clap::Parser;

use error::{Error, Result};

/// Listen for changes on the upstream for the currently checked out branch,
/// and when a change is found, pull them and run the given command
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The polling period for checking for upstream changes (in seconds)
    #[arg(default_value = "5", long, short)]
    period: f64,

    /// If enabled, brun will stop pulling new changes if `cmd` returns an error
    #[arg(long)]
    stop_on_failure: bool,

    /// The command to run on upstream changes. NOTE: this is run in a subshell
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

    fn refresh(&mut self) -> Result<Option<String>> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/commits?sha={}&per_page=1",
            self.username, self.repo, self.branch
        );
        trace!("request url={}", &url);
        let body = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()?
            .error_for_status()?;

        let commits: Vec<commit::CommitResponse> = body.json()?;
        let commit = commits.into_iter().next().ok_or(Error::GitEmptyHistory)?;
        Ok(self.last_commit.replace(commit.sha))
    }
}

fn main() {
    // Initialize the logger
    env_logger::init();

    // Parse the arguments
    let args = Args::parse();
    let user_cmd: String = args.cmd.join(" ");
    debug!("running with user command: {}", &user_cmd);

    match listen_and_run(user_cmd, args.stop_on_failure, args.period) {
        Ok(_) => {}
        Err(error) => {
            eprintln!("error: {}", error);
            std::process::exit(1);
        }
    }
}

fn listen_and_run(user_cmd: String, stop_on_failure: bool, period: f64) -> Result<()> {
    let mut state = setup()?;

    println!(
        "Listening for changes from {}/{}/{}",
        &state.username, &state.repo, &state.branch
    );

    // Refresh the state every N seconds
    loop {
        debug!("refreshing git state");
        let previous = state.refresh()?;
        println!(
            "The last commit is: {}",
            state.last_commit().unwrap_or("null")
        );
        if previous.as_deref() != state.last_commit() {
            // There was a change in the remote
            println!(
                "Remote branch changed: {} -> {}",
                previous.as_deref().unwrap_or("null"),
                state.last_commit().unwrap_or("null")
            );

            debug!("running git pull");

            // Pull the latest changes
            Command::new("git")
                .arg("pull")
                .arg("--ff-only")
                .output()
                .map_err(|_| Error::CommandFailure("git pull".to_string()))?
                .status
                .code()
                .map(|_| println!("Pulled the latest changes"))
                .ok_or(Error::CommandSignaled("git pull".to_string()))?;

            debug!("running user command");
            // Run here the user command
            let output = Command::new("sh")
                .arg("-c")
                .arg(&user_cmd)
                .output()
                .map_err(|_| Error::CommandFailure(user_cmd.clone()))?;

            // Show the output of the user command
            print!("{}", String::from_utf8_lossy(&output.stdout));
            io::stdout().flush()?;

            if !output.status.success() && stop_on_failure {
                return Err(Error::UserCommand(
                    output.status.code().unwrap_or(-1),
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
        }

        trace!("sleeping for {}", period);
        // Sleep for some time
        std::thread::sleep(Duration::from_secs_f64(period));
    }
}

/// Analyze the executing environment and collect the state
fn setup() -> Result<GithubState> {
    // Retrieve the token
    let token = std::env::var("GH_TOKEN")
        .or_else(|_| std::env::var("GITHUB_TOKEN"))
        .map_err(|_| Error::MissingToken)?;

    // Check if there are in a git repository work tree
    if !git::git_is_work_tree()? {
        return Err(Error::GitNotinWorkTree);
    }

    // TODO(alvaro): We can detect the current checked out branch
    // We can also detect the user and repo from the git config
    // But we should also allow for overriding this with flags or env
    // variables
    let repo = RemoteRepo::try_from_gitconfig()?;

    // Check if there are some unstashed changes
    if git::git_has_unstashed_changes()? {
        return Err(Error::GitDirty);
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
    username: String,
    /// The name of the repository
    repo_name: String,
    /// The name of the branch to track
    branch: String,
}

impl RemoteRepo {
    /// Initialize a RemoteRepo based on the given values or by guessing from the
    /// git configuration
    fn try_from_gitconfig() -> Result<Self> {
        // Extract the branch name
        let branch = git::git_head()?;
        debug!("found branch={}", &branch);
        // Extract the information from the upstream remote
        let (username, repo_name) = git::git_upstream_info(&branch)?;
        debug!("found username={} repo_name={}", &username, &repo_name);

        Ok(Self {
            username,
            repo_name,
            branch,
        })
    }
}
