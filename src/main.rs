use crate::config::{read_config_in_workdir, watch_config, SiostamConfig};
use crate::core::Core;
use crate::error::CustomError;
use crate::server::start_server;
use crate::subsystem_mapping::dot::generate_file_from_dot;
use crate::subsystem_mapping::Graph;
use clap::{App, Arg, SubCommand};
use dotenv::dotenv;
use env_logger::Env;
use humantime::{format_duration, parse_duration};
use log::{error, info};
use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;

mod config;
mod core;
mod error;
mod git_extraction;
mod server;
mod subsystem_mapping;

pub mod built_info {
    // The file has been placed there by the build script.
    include!(concat!(env!("OUT_DIR"), "/built.rs"));
}

#[actix_rt::main]
async fn main() {
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
                .default_value("Siostam.toml"),
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
        if let Err(err) = run_server(config_path).await {
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
    let config: SiostamConfig = read_config_in_workdir(config_path)?;

    let graph = Graph::construct_from_config(&config)?;

    graph.output_to_json("data/output.json")?;

    info!("Proceeding to generate the dot file.");

    graph.output_to_dot("data/output.dot")?;

    info!("Proceeding to generate the svg file.");

    generate_file_from_dot("data/output.dot");

    info!("Finished.");
    Ok(())
}

async fn run_server(config_path: &str) -> Result<(), CustomError> {
    // Update interval
    let duration = env::var("SIOSTAM_INTERVAL_BETWEEN_UPDATES").unwrap_or_else(|e| {
        log::error!(
            "While retrieving SIOSTAM_INTERVAL_BETWEEN_UPDATES env var: {}",
            e
        );
        "5min".to_string()
    });
    let interval_between_updates: Duration =
        parse_duration(duration.as_str()).unwrap_or_else(|e| {
            log::error!(
                "While parsing SIOSTAM_INTERVAL_BETWEEN_UPDATES env var: {}",
                e
            );
            Duration::from_secs(5 * 60)
        });
    log::info!(
        "Interval between updates: {}",
        format_duration(interval_between_updates).to_string()
    );

    // Read the configuration and access a first state of the graph
    let core = Core::new(config_path, interval_between_updates)?;
    let access_to_core = Arc::new(core);

    // Watch for changes of the configuration
    watch_config(access_to_core.clone(), config_path);

    // Run the server on current thread
    start_server(access_to_core).await?;
    Ok(())
}

fn init() -> Result<(), CustomError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open("Siostam.toml")
        .map_err(|e| CustomError::new(format!("While creating the Siostam.toml file: {}", e)))?
        .write_all(include_bytes!("../Siostam.example.toml"))
        .map_err(|e| CustomError::new(format!("While writing to the Siostam.toml file: {}", e)))?;

    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(".env")
        .map_err(|e| CustomError::new(format!("While creating the .env file: {}", e)))?
        .write_all(include_bytes!("../.env.example"))
        .map_err(|e| CustomError::new(format!("While writing to the .env file: {}", e)))?;

    Ok(())
}
