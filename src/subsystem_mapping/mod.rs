use crate::git_extraction::extraction::SubsystemFile;
use crate::subsystem_mapping::references::ReferenceByIndex;
use std::{fs, io};

use serde_derive::Deserialize;

// Structure used to avoid refcount
mod references;

// -- Models in source files --
// The models stored in files

#[derive(Debug, Deserialize)]
pub struct SubsystemFileSource {
    stored_in_system: Option<String>,
    system: Option<SystemSource>,

    // Stored as both subsystem and subsystems to handle both naming-conventions
    subsystem: Option<Vec<SubsystemSource>>,
    subsystems: Option<Vec<SubsystemSource>>,

    path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SystemSource {
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SubsystemSource {
    id: Option<String>,
    name: Option<String>,
    description: Option<String>,

    // Stored as both dependency and dependencies to handle both naming-conventions
    dependency: Option<Vec<SubsystemDependencySource>>,
    dependencies: Option<Vec<SubsystemDependencySource>>,
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

        Some(System {
            // If there is no id, use the name as backup
            id: system.id.as_ref().or(system.name.as_ref()).unwrap().clone(),

            // If there is no name, use the id as backup
            name: system.name.as_ref().or(system.id.as_ref()).unwrap().clone(),

            // Simple metadata
            description: system.description.clone(),

            // If specified, the system will be added to the parent system
            // This will be done later because all files must be extracted before
            parent_system: self
                .stored_in_system
                .as_ref()
                .map(|s| ReferenceByIndex::new(s)),

            // If specified, the system will be added to the parent system
            // This will be done later because all files must be extracted before
            systems: Vec::new(),
            subsystems: Vec::new(),
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

                // Simple metadata
                description: subsystem.description.clone(),

                // If specified, the system will be added to the parent system
                // The parent system is decided before this method is call
                // It is either the file system if there is one, or stored_in_system
                parent_system: parent_system.map(|p| ReferenceByIndex::new(p)),

                // The previously computed dependencies
                dependencies,
            });
        }

        subsystems
    }
}

// -- Post-processed models --
// The models transformed for usage in graphs

#[derive(Debug)]
pub struct System {
    id: String,
    name: String,
    description: Option<String>,

    systems: Vec<ReferenceByIndex<System>>,
    subsystems: Vec<ReferenceByIndex<Subsystem>>,

    parent_system: Option<ReferenceByIndex<System>>,
}

#[derive(Debug)]
pub struct Subsystem {
    id: String,
    name: String,
    description: Option<String>,

    dependencies: Vec<SubsystemDependency>,

    parent_system: Option<ReferenceByIndex<System>>,
}

#[derive(Debug)]
pub struct SubsystemDependency {
    subsystem: ReferenceByIndex<Subsystem>,
    why: Option<String>,
}

#[derive(Debug)]
pub struct Graph {
    systems: Vec<System>,
    subsystems: Vec<Subsystem>,
}

/// Read the content and parse it as TOML
pub fn read_file(subsystem_file: &SubsystemFile) -> io::Result<SubsystemFileSource> {
    let content: String = fs::read_to_string(&subsystem_file.path)?;
    let mut content: SubsystemFileSource = toml::from_str(content.as_str())?;

    content.path = subsystem_file
        .relative_path
        .clone()
        .into_os_string()
        .into_string()
        .ok();
    Ok(content)
}

/// Read the files and reconstruct the whole graph from them
pub fn source_to_graph(files: Vec<SubsystemFile>) -> io::Result<Graph> {
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
