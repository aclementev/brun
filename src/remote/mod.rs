pub mod github;

use crate::error::Result;

pub(crate) trait Remote {
    fn new(username: String, repo_name: String, branch: String, token: String) -> Self;
    fn last_commit(&self) -> Option<&str>;
    fn refresh(&mut self) -> Result<Option<String>>;
}
