use crate::config::{read_config_in_workdir, SubsystemMapperConfig};
use crate::git_extraction::extraction::extract_files_from_repo;
use crate::git_extraction::{get_git_repo_ready_for_extraction, get_name_from_url};
use crate::subsystem_mapping::source_to_graph;
use env_logger::Env;
use log::info;

mod config;
mod git_extraction;
mod subsystem_mapping;

fn main() {
    // Initialise the logger with INFO level by default.
    let logger_config = Env::default().default_filter_or("info");
    env_logger::from_env(logger_config).init();

    // Retrieve the list of all remotes to fetch from the config
    let config: SubsystemMapperConfig = read_config_in_workdir();

    // Get the data files
    let mut list = Vec::new();
    for target in config.targets {
        // Update/clone the repositories
        let repo_name = get_name_from_url(target.url.as_str());
        let path = get_git_repo_ready_for_extraction(&target, repo_name, config.auth.as_ref());

        // Walk in the repositories to find the files
        list.append(&mut extract_files_from_repo(
            path.as_path(),
            repo_name,
            config.suffix.as_str(),
        ));
    }
    info!("Found {} file(s)", list.len());

    // Post-process the data
    let graph = source_to_graph(list)
        .expect("Error when generating the graph");
    info!("{:#?}", graph);

    graph.output_to_json("data/output.json")
        .expect("Error when generating the json output");
}
