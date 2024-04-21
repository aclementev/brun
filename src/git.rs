use anyhow::Context;
use std::process::Command;

/// Helpers to work with a git repository
pub(crate) fn git_head() -> anyhow::Result<String> {
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
pub(crate) fn git_upstream_info(branch: &str) -> anyhow::Result<(String, String)> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("--symbolic-full-name")
        .arg(format!("{}@{{upstream}}", branch))
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

/// Check if a repository has unstashed changes, which would avoid pulling
pub(crate) fn git_has_unstashed_changes() -> anyhow::Result<bool> {
    Command::new("git")
        .arg("diff")
        .arg("--quiet")
        .status()
        .context("failed to execute git diff")?
        .code()
        .ok_or(anyhow::anyhow!("git diff exited unsuccessfully"))
        .map(|c| c != 0)
}

/// Check if we are in a git repository work tree (not `.git`)
pub(crate) fn git_is_work_tree() -> anyhow::Result<bool> {
    Command::new("git")
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .status()
        .context("failed to execute git rev-parse --is-inside-work-tree")?
        .code()
        .ok_or(anyhow::anyhow!("git rev-parse exited unsuccessfully"))
        .map(|c| c == 0)
}
