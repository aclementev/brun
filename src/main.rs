mod commit;
mod error;
mod git;
mod remote;

use log::{debug, trace};
use std::io::{self, Write};
use std::{process::Command, time::Duration};

use clap::Parser;

use error::{Error, Result};
use remote::{github, Remote};

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
    let mut remote: github::Github = setup()?;

    println!(
        "Listening for changes from {}/{}/{}",
        &remote.username, &remote.repo, &remote.branch
    );

    // Refresh the state every N seconds
    loop {
        debug!("refreshing git remote state");
        let previous = remote.refresh()?;
        println!(
            "The last commit is: {}",
            remote.last_commit().unwrap_or("null")
        );
        if previous.as_deref() != remote.last_commit() {
            // There was a change in the remote
            println!(
                "Remote branch changed: {} -> {}",
                previous.as_deref().unwrap_or("null"),
                remote.last_commit().unwrap_or("null")
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
fn setup<T: Remote>() -> Result<T> {
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

    // Create the remote
    let remote = T::new(
        repo.username.clone(),
        repo.repo_name.clone(),
        repo.branch.clone(),
        token.to_string(),
    );

    Ok(remote)
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
