use log::trace;

use crate::commit;
use crate::error::{Error, Result};
use crate::remote::Remote;

#[derive(Debug)]
pub struct Github {
    pub username: String,
    pub repo: String,
    pub branch: String,
    token: String,
    client: reqwest::blocking::Client,
    last_commit: Option<String>,
}

impl Remote for Github {
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

    fn last_commit(&self) -> Option<&str> {
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
