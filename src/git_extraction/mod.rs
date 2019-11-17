use crate::config::{Target, AuthConfig};
use crate::git_extraction::git::{open_and_update_or_clone_repo, reset_to_branch, provide_callbacks};
use git2::Repository;
use std::path::Path;

mod git;

pub fn extract_subsystems_from_target(target: &Target, auth: Option<&AuthConfig>) {
    let path = format!("data/{}", target.name);
    let path = Path::new(path.as_str());

    // Prepare the repository for extraction
    let callbacks = provide_callbacks(auth);
    let repo: Repository = open_and_update_or_clone_repo(target.url.as_str(), path, callbacks);
    reset_to_branch(target.branch.as_ref(), &repo);
}
