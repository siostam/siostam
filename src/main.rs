use crate::config::{read_config_in_workdir, SubsystemMapperConfig};
use crate::error::CustomError;
use crate::server::start_server;
use crate::subsystem_mapping::dot::generate_file_from_dot;
use crate::subsystem_mapping::{Graph, GraphRepresentation};
use clap::{App, Arg, SubCommand};
use dotenv::dotenv;
use env_logger::Env;
use log::{error, info};
use std::sync::{Arc, RwLock};

mod config;
mod error;
mod git_extraction;
mod server;
mod subsystem_mapping;

fn main() {
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
    } else {
        if let Err(err) = run_mapper(config_path) {
            error!("{}", err);
        }
    }
}

fn run_mapper(config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Retrieve the list of all remotes to fetch from the config
    let config: SubsystemMapperConfig = read_config_in_workdir(config_path)?;

    let graph = Graph::construct_from_config(&config)?;

    graph.output_to_json("data/output.json")?;

    info!("Proceeding to generate the dot file.");

    graph.output_to_dot("data/output.dot")?;

    info!("Proceeding to generate the svg file.");

    generate_file_from_dot("data/output.dot");

    info!("Finished.");
    Ok(())
}

fn run_server(config_path: &str) -> Result<(), CustomError> {
    // Retrieve the list of all remotes to fetch from the config
    let config: SubsystemMapperConfig = read_config_in_workdir(config_path)?;

    let graph = Graph::construct_from_config(&config)
        .map_err(|err| CustomError::new(format!("While constructing graph: {}", err)))?;

    let graph_representation = GraphRepresentation::from(graph)?;
    let shared_graph = Arc::new(RwLock::from(graph_representation));

    start_server(shared_graph)?;
    Ok(())
}
