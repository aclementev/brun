use thiserror::Error;

pub type Error = BrunError;
pub type Result<T> = core::result::Result<T, Error>;

// TODO(alvaro): Decide which of these errors are internal errors, and move
// them into log messages and have a generic internal error to show to the user
#[derive(Error, Debug)]
pub(crate) enum BrunError {
    #[error("you must set the GH_TOKEN or GITHUB_TOKEN environment variable")]
    MissingToken,
    #[error("there are uncommitted changes. Run `git commit`or `git stash` to save the changes and try again.")]
    GitDirty,
    #[error("remote repository has no commits")]
    GitEmptyHistory,
    #[error("you are not in a git repository (or you are inside the .git directory)")]
    GitNotinWorkTree,
    #[error("failed to retrieve HEAD branch (code={0}): {1}")]
    GitNoHead(i32, String),
    #[error("failed to get upstream branch (code={0}): {1}")]
    GitNoUpstream(i32, String),
    #[error("failed to get remote url (code={0}): {1}")]
    GitNoUpstreamURL(i32, String),
    #[error("could not get remote name from upstream branch: {0}")]
    GitBadRemote(String),

    // Execution Failure
    #[error("user command failed (code={0}): {1}")]
    UserCommand(i32, String),

    #[error("failed to start command: {0}")]
    CommandFailure(String),
    #[error("command stopped without status due to signal: {0}")]
    CommandSignaled(String),
    #[error("IOError: {0}")]
    IOError(#[from] std::io::Error),

    #[error("failed to request API: {0}")]
    APIError(#[from] reqwest::Error),

    #[error("error: {0}")]
    InternalError(String),
}
