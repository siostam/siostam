use crate::config::{read_config_in_workdir, SubsystemMapperConfig};
use crate::git_extraction::open_or_clone_repo;
use std::path::Path;

mod config;
mod git_extraction;

fn main() {
    // The list of all remotes to fetch is stored in the config
    let config: SubsystemMapperConfig = read_config_in_workdir();

    for remote in config.remotes {
        let path = format!("data/{}", remote.name);
        let path = Path::new(path.as_str());

        let repo = open_or_clone_repo(remote.url.as_str(), path);
    }
}
