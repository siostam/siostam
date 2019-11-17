use crate::config::{read_config_in_workdir, SubsystemMapperConfig};
use crate::git_extraction::open_or_clone_repo;
use env_logger::Env;
use std::path::Path;
use git2::{BranchType, Repository, Branches};
use log::info;

mod config;
mod git_extraction;

fn main() {
    env_logger::from_env(Env::default().default_filter_or("info")).init();

    // The list of all remotes to fetch is stored in the config
    let config: SubsystemMapperConfig = read_config_in_workdir();

    for remote in config.remotes {
        let path = format!("data/{}", remote.name);
        let path = Path::new(path.as_str());

        let repo: Repository = open_or_clone_repo(remote.url.as_str(), path);

        let branch_full_name = format!("refs/remotes/origin/{}", remote.branch.as_str());
        match repo.set_head(branch_full_name.as_str()) {
            Ok(()) => info!("Set head to branch"),
            Err(e) => panic!("Failed to set head: {}", e),
        }
    }
}
