use crate::config::SubsystemMapperConfig;
use crate::error::CustomError;
use crate::git_extraction::extraction::{extract_files_from_repo, SubsystemFile};
use crate::git_extraction::{get_git_repo_ready_for_extraction, get_name_from_url};
use crate::subsystem_mapping::dot::{generate_file_from_dot, DotBuilder};
use crate::subsystem_mapping::references::ReferenceByIndex;
use log::{debug, error, info, warn};
use serde_derive::{Deserialize, Serialize};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::path::PathBuf;
use std::{fs, io};

// Structure used to avoid refcount
mod references;
// Output in dot format
pub mod dot;

// -- Models in source files --
// The models stored in files

#[derive(Debug, Deserialize)]
pub struct SubsystemFileSource {
    stored_in_system: Option<String>,
    system: Option<SystemSource>,

    // Stored as both subsystem and subsystems to handle both naming-conventions
    subsystem: Option<Vec<SubsystemSource>>,
    subsystems: Option<Vec<SubsystemSource>>,

    // It is stored as Option because it is added by code, but we can unwrap it safely
    repo_name: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SystemSource {
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,

    // Stored as both how_to and howto to handle both naming-conventions
    howto: Option<Vec<HowToSource>>,
    how_to: Option<Vec<HowToSource>>,
}

#[derive(Debug, Deserialize)]
pub struct SubsystemSource {
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,

    // Stored as both dependency and dependencies to handle both naming-conventions
    dependency: Option<Vec<SubsystemDependencySource>>,
    dependencies: Option<Vec<SubsystemDependencySource>>,
    // Stored as both how_to and howto to handle both naming-conventions
    howto: Option<Vec<HowToSource>>,
    how_to: Option<Vec<HowToSource>>,
}

#[derive(Debug, Deserialize)]
pub struct HowToSource {
    url: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubsystemDependencySource {
    id: Option<String>,
    why: Option<String>,
}

// -- Transformation --

/// In some cases, we have two vecs (for instance dependency and dependencies) and we want to
/// iterate over both as if it was only one vec.
fn iterate_over_option_vecs<'a, T>(
    vec_a: &'a Option<Vec<T>>,
    vec_b: &'a Option<Vec<T>>,
) -> impl Iterator<Item = &'a T> {
    // For each vec, we do a first iter to unwrap the option
    // Then we do a flat_map over another iter in order to lay out the vec
    let iter_a = vec_a.iter().flat_map(|v| v.iter());
    let iter_b = vec_b.iter().flat_map(|v| v.iter());

    // Finally we put the iterator in sequence, and TADAAAA!
    iter_a.chain(iter_b)
}

impl SubsystemFileSource {
    /// Get a fully checked system out of the file
    /// If invalid, None is returned
    pub fn extract_system(&self) -> Option<System> {
        // This case is pretty obvious, don't you think
        if self.system.is_none() {
            return None;
        }

        // If we don't have neither name nor id, it can't be valid either
        let system = self.system.as_ref().unwrap();
        if system.id.is_none() && system.name.is_none() {
            return None;
        }

        // Process the related how-to
        let mut how_to_vec = Vec::new();
        for how_to in iterate_over_option_vecs(&system.how_to, &system.howto) {
            if how_to.url.is_some() {
                how_to_vec.push(HowTo {
                    url: how_to.url.as_ref().unwrap().clone(),
                    text: how_to
                        .text
                        .as_ref()
                        .or(how_to.url.as_ref())
                        .unwrap()
                        .clone(),
                })
            }
        }

        Some(System {
            // If there is no id, use the name as backup
            id: system.id.as_ref().or(system.name.as_ref()).unwrap().clone(),

            // If there is no name, use the id as backup
            name: system.name.as_ref().or(system.id.as_ref()).unwrap().clone(),

            // Store the repo_name/path to display it on the front-end
            repo_name: self.repo_name.clone().unwrap(),
            path: self.path.clone().unwrap(),

            // Simple metadata
            description: system.description.clone(),

            // If specified, the system will be added to the parent system
            // This will be done later because all files must be extracted before
            parent_system: self
                .stored_in_system
                .as_ref()
                .map(|s| ReferenceByIndex::new(s)),

            how_to: how_to_vec,
        })
    }

    /// Get a valid subsystems from a file
    /// Invalid subsystems are ignored
    pub fn extract_subsystems(&self, parent_system: Option<&String>) -> Vec<Subsystem> {
        let mut subsystems = Vec::new();

        // Iterate over both subsystem and subsystems to handle both naming-conventions
        for subsystem in iterate_over_option_vecs(&self.subsystems, &self.subsystem) {
            // If we don't have neither name nor id, it can't be valid
            if subsystem.id.is_none() && subsystem.name.is_none() {
                continue;
            }

            // Process the dependencies. It doesn't search for indexes yet.
            let mut dependencies = Vec::new();
            for dependency in
                iterate_over_option_vecs(&subsystem.dependencies, &subsystem.dependency)
            {
                if dependency.id.is_some() {
                    dependencies.push(SubsystemDependency {
                        subsystem: ReferenceByIndex::new(dependency.id.as_ref().unwrap()),
                        why: dependency.why.clone(),
                    })
                }
            }

            // Process the related how-to
            let mut how_to_vec = Vec::new();
            for how_to in iterate_over_option_vecs(&subsystem.how_to, &subsystem.howto) {
                if how_to.url.is_some() {
                    how_to_vec.push(HowTo {
                        url: how_to.url.as_ref().unwrap().clone(),
                        text: how_to
                            .text
                            .as_ref()
                            .or(how_to.url.as_ref())
                            .unwrap()
                            .clone(),
                    })
                }
            }

            subsystems.push(Subsystem {
                // If there is no id, use the name as backup
                id: subsystem
                    .id
                    .as_ref()
                    .or(subsystem.name.as_ref())
                    .unwrap()
                    .clone(),

                // If there is no name, use the id as backup
                name: subsystem
                    .name
                    .as_ref()
                    .or(subsystem.id.as_ref())
                    .unwrap()
                    .clone(),

                // Store the repo_name/path to display it on the front-end
                repo_name: self.repo_name.clone().unwrap(),
                path: self.path.clone().unwrap(),

                // Simple metadata
                description: subsystem.description.clone(),

                // If specified, the system will be added to the parent system
                // The parent system is decided before this method is call
                // It is either the file system if there is one, or stored_in_system
                parent_system: parent_system.map(|p| ReferenceByIndex::new(p)),

                // The previously computed dependencies
                dependencies,
                how_to: how_to_vec,
            });
        }

        subsystems
    }
}

// -- Post-processed models --
// The models transformed for usage in graphs

#[derive(Debug, Serialize)]
pub struct System {
    id: String,
    name: String,
    repo_name: String,
    path: String,
    description: Option<String>,

    parent_system: Option<ReferenceByIndex<System>>,

    how_to: Vec<HowTo>,
}

#[derive(Debug, Serialize)]
pub struct Subsystem {
    id: String,
    name: String,
    repo_name: String,
    path: String,
    description: Option<String>,

    parent_system: Option<ReferenceByIndex<System>>,

    dependencies: Vec<SubsystemDependency>,
    how_to: Vec<HowTo>,
}

#[derive(Debug, Serialize)]
pub struct HowTo {
    url: String,
    text: String,
}

#[derive(Debug, Serialize)]
pub struct SubsystemDependency {
    subsystem: ReferenceByIndex<Subsystem>,
    why: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Graph {
    systems: Vec<System>,
    subsystems: Vec<Subsystem>,
}

impl Graph {
    pub fn construct_from_config(
        config: &SubsystemMapperConfig,
    ) -> Result<Graph, Box<dyn std::error::Error>> {
        // Get the data files
        let mut list = Vec::new();
        for target in config.targets.iter() {
            // The path can be automatic (git repo) or local
            let path: PathBuf;
            let repo_name: String;

            if target.folder.is_some() {
                path = PathBuf::from(target.folder.as_ref().unwrap());
                repo_name = path.as_os_str().to_string_lossy().to_string();

                if !path.exists() {
                    return Err(Box::from(CustomError::new(format!(
                        "Local folder does not exists"
                    ))));
                } else {
                    // The local folder mode is useful to quickly view the result but it is rather
                    // error-prone. Displays warning to make sure the user knows it is located in local.
                    warn!("Opened local folder {}", path.display());
                }
            } else if target.url.is_some() && target.branch.is_some() {
                // Update/clone the repositories
                let url = target.url.as_ref().unwrap();
                let branch = target.branch.as_ref().unwrap();
                repo_name = get_name_from_url(url.as_str()).to_owned();
                path = get_git_repo_ready_for_extraction(&url, &branch, &repo_name)?;
            } else {
                error!("Target must have 'url' + 'branch' or 'folder'. Neither is available here");
                continue;
            };

            // Walk in the repositories to find the files
            list.append(&mut extract_files_from_repo(
                path.as_path(),
                &repo_name,
                config.suffix.as_str(),
            ));
        }
        info!("Found {} file(s)", list.len());

        // Post-process the data
        let graph = source_to_graph(list)?;
        debug!("{:#?}", graph);
        Ok(graph)
    }

    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Outputs all the data as JSON for the front-end
    pub fn output_to_json(&self, path: &str) -> serde_json::Result<()> {
        fs::write(path, self.to_json()?).expect("Error with the json output");
        Ok(())
    }

    /// Output the graph as DOT
    pub fn output_to_dot(&self, path: &str) -> io::Result<()> {
        let mut dot = DotBuilder::new(path)?;
        let indent = "  ";

        // Generate the systems + subsystems, but not the edges.
        // The edges must be at the root because an edge can't link something outside the cluster
        // That's why the links are added at root

        // 1. Recursively generate systems (clusters) and subsystems (nodes)
        self.output_system(&mut dot, None, indent)?;
        // 2. Add subsystems' dependencies (edges)
        self.output_subsystems_dependencies(&mut dot, indent)?;

        // Print the end of file and close it
        dot.close()?;

        Ok(())
    }

    /// Recursively output systems and subsytems as DOT
    fn output_system(
        &self,
        mut dot: &mut DotBuilder,
        current_parent_index: Option<usize>,
        indent: &str,
    ) -> io::Result<()> {
        // 1. We search for systems with a given parent
        // We begin with current_parent_index = None, which is the root of the graph
        for (index, system) in self.systems.iter().enumerate() {
            // Is the system targeted by this call of output_system?
            let parent_system_index = system.parent_system.as_ref().and_then(|p| p.index());
            if parent_system_index == current_parent_index {
                // Begin a new cluster
                dot.begin_cluster(&indent, &system.id, &system.name);

                // Display children systems
                self.output_system(&mut dot, Some(index), format!("{}  ", indent).as_str())?;

                // Close the cluster
                dot.end_cluster(&indent);
            }
        }

        // 2. We search for subsystems with a given parent
        for subsystem in self.subsystems.iter() {
            // Again, we use the parent_system index to find if it is targeted or not
            let parent_system_index = subsystem.parent_system.as_ref().and_then(|p| p.index());
            if parent_system_index == current_parent_index {
                dot.add_node(&indent, &subsystem.id, &subsystem.name);
            }
        }

        Ok(())
    }

    /// Print dependencies between subsystems as DOT
    fn output_subsystems_dependencies(&self, dot: &mut DotBuilder, indent: &str) -> io::Result<()> {
        // Parse all subsystems dependencies
        for subsystem_a in self.subsystems.iter() {
            for dependency in subsystem_a.dependencies.iter() {
                // Search for the targeted system. If there is one output it
                if let Some(subsystem_b) = dependency.subsystem.index().map(|s| &self.subsystems[s])
                {
                    dot.add_edge(&indent, &subsystem_a.id, &subsystem_b.id);
                }
            }
        }

        Ok(())
    }
}

/// Read the content and parse it as TOML
pub fn read_file(subsystem_file: &SubsystemFile) -> Result<SubsystemFileSource, CustomError> {
    let content: String = fs::read_to_string(&subsystem_file.path).map_err(|err| {
        CustomError::new(format!(
            "While reading subsystem file `{:?}`: {}",
            subsystem_file.path, err
        ))
    })?;
    let mut content: SubsystemFileSource = toml::from_str(content.as_str()).map_err(|err| {
        CustomError::new(format!(
            "While parsing subsystem file as TOML `{:?}`: {}",
            subsystem_file.path, err
        ))
    })?;

    content.repo_name = Some(subsystem_file.repo_name.clone());
    content.path = Some(subsystem_file.relative_path.clone());
    Ok(content)
}

/// Read the files and reconstruct the whole graph from them
pub fn source_to_graph(files: Vec<SubsystemFile>) -> Result<Graph, CustomError> {
    // First, we read the files and store each system, subsystem
    let mut graph = merge_all_files(files)?;

    // Then, we use the ids to link system and subsystems together
    reconstruct_links(&mut graph);

    Ok(graph)
}

/// Get all systems/subsystems from the files
fn merge_all_files(files: Vec<SubsystemFile>) -> Result<Graph, CustomError> {
    // Read the content of the files as TOML
    let files: Result<Vec<_>, _> = files.iter().map(|f| read_file(f)).collect();
    let files = files?;

    // WARNING: items in these Vec<> must only be added at the end to preserve indexes.
    let mut systems: Vec<System> = Vec::new();
    let mut subsystems: Vec<Subsystem> = Vec::new();

    // Process each file
    for file in files {
        // First we need the system.
        // If there is one specified, it will be considered as the subsystems parent
        let system = file.extract_system();

        // Get the id of the local parent for the subsystems:
        //  - the system if there is one
        //  - the stored_in_system if present
        //  - or none
        let system_id = system.as_ref().map(|s| &s.id);
        let subsystem_parent = system_id.or(file.stored_in_system.as_ref());

        // Get the subsystems
        let mut local_subsystems: Vec<Subsystem> = file.extract_subsystems(subsystem_parent);

        // Add the systems/subsystems to the list
        if system.is_some() {
            systems.push(system.unwrap());
        }
        subsystems.append(&mut local_subsystems);
    }

    Ok(Graph {
        systems,
        subsystems,
    })
}

// Parse each ReferenceByIndex and search for the target in the graph
fn reconstruct_links(unlinked_graph: &mut Graph) {
    // Construct indexes
    let mut systems = HashMap::with_capacity(unlinked_graph.systems.len());
    let mut subsystems = HashMap::with_capacity(unlinked_graph.subsystems.len());

    // TODO: handle conflicts
    for (index, system) in unlinked_graph.systems.iter().enumerate() {
        systems.insert(system.id.clone(), index);
    }
    for (index, subsystem) in unlinked_graph.subsystems.iter().enumerate() {
        subsystems.insert(subsystem.id.clone(), index);
    }

    // Use these indexes to construct the links
    // 1. For parent systems
    unlinked_graph
        .systems
        .iter_mut()
        .filter_map(|s| s.parent_system.as_mut())
        .for_each(|parent| parent.find_index_in(&systems));
    unlinked_graph
        .subsystems
        .iter_mut()
        .filter_map(|s| s.parent_system.as_mut())
        .for_each(|parent| parent.find_index_in(&systems));

    // 2. For subsystems' dependencies
    unlinked_graph
        .subsystems
        .iter_mut()
        .flat_map(|s: &mut Subsystem| s.dependencies.iter_mut())
        .map(|dep: &mut SubsystemDependency| dep.subsystem.borrow_mut())
        .for_each(|parent: &mut ReferenceByIndex<Subsystem>| parent.find_index_in(&subsystems));
}

pub struct GraphRepresentation {
    json: String,
    svg: String,
}

impl GraphRepresentation {
    pub fn from(graph: Graph) -> Result<GraphRepresentation, CustomError> {
        // JSON representation
        let json = graph.to_json().map_err(|err| {
            CustomError::new(format!("While constructing json representation: {}", err))
        })?;

        // DOT representation
        info!("Proceeding to generate the dot file.");
        graph.output_to_dot("data/output.dot").map_err(|err| {
            CustomError::new(format!(
                "While reading generating dot file `data/output.dot`: {}",
                err
            ))
        })?;

        // SVG representation
        info!("Proceeding to generate the svg file.");
        generate_file_from_dot("data/output.dot");
        let svg = fs::read_to_string("data/output.dot.svg").map_err(|err| {
            CustomError::new(format!(
                "While reading svg file `data/output.dot.svg`: {}",
                err
            ))
        })?;

        info!("Finished.");

        Ok(GraphRepresentation { json, svg })
    }

    pub fn json(&self) -> String {
        self.json.clone()
    }

    pub fn svg(&self) -> String {
        self.svg.clone()
    }
}
