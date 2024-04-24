use serde::Deserialize;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct UserInfo {
    name: String,
    email: String,
    date: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Tree {
    sha: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Commit {
    author: UserInfo,
    committer: UserInfo,
    message: String,
    tree: Tree,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct User {
    login: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct Ref {
    sha: String,
    url: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct CommitResponse {
    pub sha: String,
    commit: Commit,
    url: String,
    author: User,
    committer: User,
    parents: Vec<Ref>,
}
