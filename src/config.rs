use crate::core::Core;
use crate::error::CustomError;
use notify::{DebouncedEvent, Op, RawEvent, RecommendedWatcher, RecursiveMode, Watcher};
use serde_derive::Deserialize;
use std::fs;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;

// -- Structs --

/// Stores the configuration about the repository to scrap (and how to scrap them)
/// Each Target is a repository/local folder
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct SiostamConfig {
    pub(crate) suffix: String,
    pub(crate) targets: Vec<Target>,
}

/// Contains data about a repository/local folder to scrap.
/// Url and branch are used in "git repository" setting (when folder is not defined)
/// Folder points a local folder
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct Target {
    pub(crate) url: Option<String>,
    pub(crate) branch: Option<String>,
    pub(crate) folder: Option<String>,
}

// -- Methods: reading the configuration --

pub fn read_config_in_workdir(path: &str) -> Result<SiostamConfig, CustomError> {
    // Read the file
    let config: String = fs::read_to_string(path).map_err(|err| {
        CustomError::new(format!("While reading config file `{}`: {}", path, err))
    })?;

    // Parse the resulting string
    let toml = toml::from_str(config.as_str()).map_err(|err| {
        CustomError::new(format!(
            "While parsing config file `{}` as TOML: {}",
            path, err
        ))
    })?;

    // Yay, a complete config
    Ok(toml)
}

// -- Methods: watching the configuration --

/// Watch for file modification at the given path and warn the Core if there is one
pub fn watch_config(access_to_core: Arc<Core>, path: &str) {
    let path = String::from(path);

    // Set a thread to wait for change events
    thread::spawn(move || {
        if let Err(err) = watch(access_to_core, path.as_str()) {
            log::error!("While watching config file `{}`: {}", path, err)
        }
    });
}

/// Internal watch method (separated from watch_config to handle Result<>)
/// Source: https://github.com/notify-rs/notify/tree/v4.0.13#notify
fn watch(access_to_core: Arc<Core>, path: &str) -> notify::Result<Receiver<DebouncedEvent>> {
    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let mut watcher: RecommendedWatcher = Watcher::new_raw(tx)?;

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher.watch(path, RecursiveMode::NonRecursive)?;

    // This is a simple loop, but you may want to use more complex logic here,
    // for example to handle I/O.
    loop {
        match rx.recv() {
            Ok(RawEvent {
                path: Some(_path_buf),
                op: Ok(op),
                cookie,
            }) => {
                // We only care about write events
                if op == Op::WRITE {
                    match access_to_core.reload_config() {
                        Ok(()) => log::info!("Configuration reloaded"),
                        Err(err) => log::error!("While reloading configuration: {}", err),
                    }
                } else {
                    log::trace!("{:?} {:?} ({:?})", op, path, cookie);
                }
            }
            Ok(event) => log::error!("Broken watch event (for configuration): {:?}", event),
            Err(e) => log::error!("Watch error (for configuration): {:?}", e),
        }
    }
}
