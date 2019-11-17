use crate::config::{read_config_in_workdir, SubsystemMapperConfig};
use crate::git_extraction::get_git_repo_ready_for_extraction;
use env_logger::Env;
use crate::git_extraction::extraction::extract_files_from_repo;
use log::{info};

mod config;
mod git_extraction;

fn main() {
    // Initialise the logger with INFO level by default.
    let logger_config = Env::default().default_filter_or("info");
    env_logger::from_env(logger_config).init();

    // The list of all remotes to fetch is stored in the config.
    let config: SubsystemMapperConfig = read_config_in_workdir();

    for target in config.targets {
        let path = get_git_repo_ready_for_extraction(&target, config.auth.as_ref());
        let list = extract_files_from_repo(path.as_path(), config.suffix.as_str());

        info!("Found {} file(s)", list.len());
    }
}
