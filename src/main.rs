use crate::config::{read_config_in_workdir, SubsystemMapperConfig};
use crate::git_extraction::extract_subsystems_from_target;
use env_logger::Env;

mod config;
mod git_extraction;

fn main() {
    // Initialise the logger with INFO level by default.
    let logger_config = Env::default().default_filter_or("info");
    env_logger::from_env(logger_config).init();

    // The list of all remotes to fetch is stored in the config.
    let config: SubsystemMapperConfig = read_config_in_workdir();

    for target in config.targets {
        extract_subsystems_from_target(&target, config.auth.as_ref());
    }
}
