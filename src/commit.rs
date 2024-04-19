use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UserInfo {
    name: String,
    email: String,
    date: String,
}

#[derive(Debug, Deserialize)]
pub struct Tree {
    sha: String,
    url: String,
}

#[derive(Debug, Deserialize)]
pub struct Verification {
    verified: bool,
    reason: String,
    signature: Option<String>,
    payload: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Commit {
    author: UserInfo,
    committer: UserInfo,
    message: String,
    tree: Tree,
    url: String,
    comment_count: u64,
    verification: Verification,
}

#[derive(Debug, Deserialize)]
pub struct User {
    login: String,
    id: u64,
    node_id: String,
    r#type: String,
}

#[derive(Debug, Deserialize)]
pub struct Ref {
    sha: String,
    url: String,
    html_url: String,
}

#[derive(Debug, Deserialize)]
pub struct CommitResponse {
    sha: String,
    node_id: String,
    commit: Commit,
    url: String,
    html_url: String,
    author: User,
    committer: User,
    parents: Vec<Ref>,
}
