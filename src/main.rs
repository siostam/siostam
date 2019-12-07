use crate::config::{read_config_in_workdir, SubsystemMapperConfig};
use crate::error::CustomError;
use crate::server::start_server;
use crate::subsystem_mapping::dot::generate_file_from_dot;
use crate::subsystem_mapping::{Graph, GraphRepresentation};
use clap::{App, Arg, SubCommand};
use dotenv::dotenv;
use env_logger::Env;
use log::{error, info};
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Arc, RwLock};

mod config;
mod error;
mod git_extraction;
mod server;
mod subsystem_mapping;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

fn main() {
    // -- CLI setup --
    let matches = App::new(built_info::PKG_NAME)
        .version(built_info::PKG_VERSION)
        .author(built_info::PKG_AUTHORS)
        .about(built_info::PKG_DESCRIPTION)
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
        .subcommand(
            SubCommand::with_name("init")
                .about("Add the files in the local directory to get started"),
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

    // Write placeholder files if required to
    if let Some(_matches) = matches.subcommand_matches("init") {
        match init() {
            Ok(_) => info!("Initialisation complete!"),
            Err(err) => error!("{}", err),
        }
        return;
    }

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

fn init() -> Result<(), CustomError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open("SubsystemMapper.toml")
        .map_err(|e| {
            CustomError::new(format!(
                "While creating the SubsystemMapper.toml file: {}",
                e
            ))
        })?
        .write_all(include_bytes!("../SubsystemMapper.example.toml"))
        .map_err(|e| {
            CustomError::new(format!(
                "While writing to the SubsystemMapper.toml file: {}",
                e
            ))
        })?;

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(".env")
        .map_err(|e| CustomError::new(format!("While creating the .env file: {}", e)))?
        .write_all(include_bytes!("../.env.example"))
        .map_err(|e| CustomError::new(format!("While writing to the .env file: {}", e)))?;

    Ok(())
}
