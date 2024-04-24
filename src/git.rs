use std::process::Command;

use crate::error::{Error, Result};
use log::trace;

/// Helpers to work with a git repository
pub(crate) fn git_head() -> Result<String> {
    // TODO(alvaro): Make it work with an arbitrary branch
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .output()
        .map_err(|_| Error::CommandFailure("git rev-parse HEAD".to_string()))?;

    if !output.status.success() {
        return Err(Error::GitNoHead(
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Get the remote username and repo_name from the git remote information
pub(crate) fn git_upstream_info(branch: &str) -> Result<(String, String)> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("--symbolic-full-name")
        .arg(format!("{}@{{upstream}}", branch))
        .output()
        .map_err(|_| Error::CommandFailure("git diff".to_string()))?;

    if !output.status.success() {
        return Err(Error::GitNoUpstream(
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let upstream = String::from_utf8_lossy(&output.stdout).trim().to_string();
    trace!("found upstream={}", &upstream);

    // FIXME(alvaro): This fails with branches that have `/` in them
    let remote = upstream
        .rsplit('/')
        .nth(1)
        .ok_or(Error::GitBadRemote(upstream.clone()))?;

    trace!("found upstream remote={}", &remote);

    // Get the information from the remote
    let output = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg(remote)
        .output()
        .map_err(|_| Error::CommandFailure("git remote get-url <remote>".to_string()))?;

    if !output.status.success() {
        return Err(Error::GitNoUpstreamURL(
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ));
    }

    let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

    trace!("found upstream remote url={}", &url);

    // Parse the URL
    let (username, repo_name) = if url.starts_with("git@") {
        // It's an SSH URL
        let repo_uri = url
            .rsplit(':')
            .next()
            .ok_or(Error::GitBadRemote(url.clone()))?;
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
        let username = uri_parts.next().ok_or(Error::InternalError(format!(
            "the URI to have at least one slash: {}",
            url
        )))?;

        (username, repo_name)
    };

    // Trim the `.git` suffix, it it's there
    let repo_name = if repo_name.ends_with(".git") {
        trace!("repo name has .git suffix={}", &repo_name);
        repo_name.strip_suffix(".git").unwrap()
    } else {
        repo_name
    };

    Ok((username.to_string(), repo_name.to_string()))
}

/// Check if a repository has unstashed changes, which would avoid pulling
pub(crate) fn git_has_unstashed_changes() -> Result<bool> {
    Command::new("git")
        .arg("diff")
        .arg("--quiet")
        .output()
        .map_err(|_| Error::CommandFailure("git diff".to_string()))?
        .status
        .code()
        .ok_or_else(|| Error::CommandSignaled("git diff".to_string()))
        .map(|c| c != 0)
}

/// Check if we are in a git repository work tree (not `.git`)
pub(crate) fn git_is_work_tree() -> Result<bool> {
    Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .map_err(|_| Error::CommandFailure("git rev-parse --is-inside-work-tree".to_string()))?
        .status
        .code()
        .ok_or_else(|| Error::CommandSignaled("git rev-parse --is-inside-work-tree".to_string()))
        .map(|c| c == 0)
}
