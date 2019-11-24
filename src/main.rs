use crate::config::{read_config_in_workdir, SubsystemMapperConfig};
use crate::git_extraction::extraction::extract_files_from_repo;
use crate::git_extraction::{get_git_repo_ready_for_extraction, get_name_from_url};
use crate::subsystem_mapping::dot::generate_file_from_dot;
use crate::subsystem_mapping::source_to_graph;
use crate::server::start_server;
use clap::{App, Arg, SubCommand};
use env_logger::Env;
use log::{info, error};
use crate::error::CustomError;
use dotenv::dotenv;

mod config;
mod git_extraction;
mod subsystem_mapping;
mod server;
mod error;

fn main()  {
    // -- CLI setup --
    let matches = App::new("Subsystem mapper")
        .version("0.1")
        .author("Elouan Poupard-Cosquer <contact@fanaen.fr>")
        .about("Map and document systems and subsystems across multiple git repositories")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .takes_value(true)
                .default_value("SubsystemMapper.toml"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .subcommand(
            SubCommand::with_name("serve")
                .alias("server")
                .about("Start as server"),
        )
        .get_matches();

    // Load .env content into environment variables
    dotenv().ok();

    // Initialise the logger with INFO level by default.
    let default_level = match matches.occurrences_of("v") {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    let logger_config = Env::default().default_filter_or(default_level);
    env_logger::from_env(logger_config).init();

    // The config_path has a default value so we can safely unwrap it
    let config_path = matches.value_of("config").unwrap();

    if let Some(_matches) = matches.subcommand_matches("serve") {
        if let Err(err) = run_server(config_path) {
            error!("{}", err);
        }
    }
    else {
        if let Err(err) = run_mapper(config_path) {
            error!("{}", err);
        }
    }
}

fn run_mapper(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Retrieve the list of all remotes to fetch from the config
    let config: SubsystemMapperConfig = read_config_in_workdir(config_path)?;

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
    let graph = source_to_graph(list)?;
    info!("{:#?}", graph);

    graph.output_to_json("data/output.json")?;

    info!("Proceeding to generate the dot file.");

    graph.output_to_dot("data/output.dot")?;

    info!("Proceeding to generate the svg file.");

    generate_file_from_dot("data/output.dot");

    info!("Finished.");
    Ok(())
}

fn run_server(config_path: &str) -> Result<(), CustomError> {
    start_server()?;
    Ok(())
}
