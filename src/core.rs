use crate::config::{read_config_in_workdir, SiostamConfig};
use crate::error::CustomError;
use crate::subsystem_mapping::{Graph, GraphRepresentation};
use std::ops::Deref;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};

/// Store the metadata required for update checking
pub struct Updatable<T> {
    version: usize,
    last_check: Instant,
    storage: T,
    has_been_acknowledged: bool,
}

impl<T> Updatable<T>
where
    T: Eq,
{
    /// Store the first version for this storage
    ///
    /// The first version is automatically acknowledged because it is not possible to rely
    /// on a previous version
    pub fn from(storage: T) -> Updatable<T> {
        Updatable {
            version: 0,
            last_check: Instant::now(),
            storage,

            // The first version is automatically acknowledged because it is not possible
            // to rely on a previous version
            has_been_acknowledged: true,
        }
    }

    /// If a new version has been computed, you put it in storage using this method.
    /// If the new version is different, the state change will be stored so we know we have to warn
    /// the user
    pub fn update(&mut self, new_version: T) {
        let is_different = new_version != self.storage;

        if is_different {
            self.version = self.version + 1;
            self.has_been_acknowledged = false;
            self.storage = new_version;
        }

        self.last_check = Instant::now();
    }

    pub fn acknowledge(&mut self) {
        self.has_been_acknowledged = true;
    }
}

/// Core holds all the information on the graph and whether an update is required
/// Every update and every access to data goes through the core
///
/// Dev: later, it may hold cache and drive partial update to the graph
pub struct Core {
    /// How frequently we update the graph (if the file changes, this event is bypassed)
    interval_between_updates: Duration,
    /// The path to the configuration, to be able to reload later
    config_path: String,
    /// The current configuration
    config: RwLock<Updatable<SiostamConfig>>,
    /// The current graph data
    graph: RwLock<Updatable<GraphRepresentation>>,
    /// Is a graph update in progress
    is_graph_updating: Arc<Mutex<()>>,
}

impl Core {
    /// Read the config, construct a first graph and store data required to watch for changes
    pub fn new(config_path: &str, interval_between_updates: Duration) -> Result<Core, CustomError> {
        // Retrieve the list of all remotes to fetch from the config
        let config: SiostamConfig = read_config_in_workdir(config_path)?;

        let graph = Graph::construct_from_config(&config)
            .map_err(|err| CustomError::new(format!("While constructing graph: {}", err)))?;

        let graph_representation = GraphRepresentation::from(graph)?;

        Ok(Core {
            interval_between_updates,
            config_path: config_path.to_string(),
            config: RwLock::from(Updatable::from(config)),
            graph: RwLock::from(Updatable::from(graph_representation)),
            is_graph_updating: Arc::new(Mutex::from(())),
        })
    }

    // -- Updates --

    /// Check for a new version of the configuration. Usually triggered by a change in file
    pub fn reload_config(&self) -> Result<(), CustomError> {
        let config: SiostamConfig = read_config_in_workdir(self.config_path.as_str())?;

        let mut pointer_to_config = self
            .config
            .write()
            .map_err(|err| CustomError::new(format!("While reloading configuration: {}", err)))?;

        log::debug!("New config: {:?}", config);
        (*pointer_to_config).update(config);

        Ok(())
    }

    /// Do an update if the timer is up or if the config changed
    /// Contains a security to avoid doing multiple update at once
    pub fn check_for_graph_update(core: Arc<Core>) -> Result<(), CustomError> {
        if !core.is_graph_update_required()? {
            return Ok(());
        }

        // If we already are updating, don't do it twice
        if core.is_graph_updating.try_lock().is_err() {
            return Ok(());
        }

        // Do it in another thread
        thread::spawn(move || {
            log::info!("Starting graph update");
            match core.upgrade_graph() {
                Ok(()) => log::info!("Graph update complete"),
                Err(err) => log::error!("While updating graph: {}", err),
            }
        });

        Ok(())
    }

    fn is_graph_update_required(&self) -> Result<bool, CustomError> {
        let config = self.config.read().map_err(|e| {
            CustomError::new(format!("While accessing the in-memory config: {}", e))
        })?;

        let graph = self
            .graph
            .read()
            .map_err(|e| CustomError::new(format!("While accessing the in-memory graph: {}", e)))?;

        // If the config changed or if the graph has been updated since a while, "yes, please update"
        Ok(!config.has_been_acknowledged
            || graph.last_check.elapsed() > self.interval_between_updates)
    }

    pub fn version(&self) -> Result<usize, CustomError> {
        let graph = self
            .graph
            .read()
            .map_err(|e| CustomError::new(format!("While accessing the in-memory graph: {}", e)))?;

        Ok(graph.version)
    }

    /// Use the current config and proceed to update the whole graph
    fn upgrade_graph(&self) -> Result<(), CustomError> {
        if let Ok(_guard) = self.is_graph_updating.clone().lock() {
            // Access the current config
            let mut config = self.config.write().map_err(|e| {
                CustomError::new(format!("While accessing the in-memory config: {}", e))
            })?;

            // Construct the graph
            let graph = Graph::construct_from_config(&(*config).storage)
                .map_err(|err| CustomError::new(format!("While constructing graph: {}", err)))?;

            // Regenerate JSON/SVG
            let graph_representation = GraphRepresentation::from(graph)?;

            let mut graph_storage = self.graph.write().map_err(|e| {
                CustomError::new(format!(
                    "While editing the in-memory graph representation: {}",
                    e
                ))
            })?;

            (*config).acknowledge();
            (*graph_storage).update(graph_representation);
        }

        Ok(())
    }

    // -- Getters --

    /// Read the current version of the graph
    pub fn json(&self) -> Result<String, CustomError> {
        let lock = self
            .graph
            .read()
            .map_err(|e| CustomError::new(format!("While accessing the in-memory json: {}", e)))?;

        Ok(lock.deref().storage.json())
    }

    /// Read the current version of the graph
    pub fn svg(&self) -> Result<String, CustomError> {
        let lock = self
            .graph
            .read()
            .map_err(|e| CustomError::new(format!("While accessing the in-memory svg: {}", e)))?;

        Ok(lock.deref().storage.svg())
    }
}
