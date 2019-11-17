use crate::config::{read_config_in_workdir, SubsystemMapperConfig};
use crate::git_extraction::open_or_clone_repo;
use env_logger::Env;
use std::path::Path;
use git2::{BranchType, Repository, Branches, Remote, ResetType, Branch};
use log::info;

mod config;
mod git_extraction;

fn main() {
    env_logger::from_env(Env::default().default_filter_or("info")).init();

    // The list of all remotes to fetch is stored in the config
    let config: SubsystemMapperConfig = read_config_in_workdir();

    for target in config.targets {
        let path = format!("data/{}", target.name);
        let path = Path::new(path.as_str());

        let repo: Repository = open_or_clone_repo(target.url.as_str(), path);

        let branch_name = format!("origin/{}", target.branch);
        let branch: Branch = repo.find_branch(branch_name.as_ref(), BranchType::Remote)
            .expect("Branch not found");
        let mut remote: Remote = repo.find_remote("origin")
            .expect("You have no origin?");

        remote.download(&[], None)
            .expect("Error when downloading");
        remote.disconnect();

        let branch_full_name = format!("refs/remotes/origin/{}", target.branch.as_str());
        let branch_object = branch.get().peel_to_commit().expect("Commit not found");
        match repo.reset(branch_object.as_object(), ResetType::Hard, None) {
            Ok(()) => info!("Set head to branch"),
            Err(e) => panic!("Failed to set head: {}", e),
        }
    }
}
