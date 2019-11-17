use log::info;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct SubsystemFile {
    pub name: String,
    pub path: PathBuf,
}

/// List all files in repository with a name ending by the given suffix
pub fn extract_files_from_repo(path: &Path, suffix: &str) -> Vec<SubsystemFile> {
    let mut file_list: Vec<SubsystemFile> = Vec::new();

    for entry in WalkDir::new(path) {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy();

        if name.ends_with(suffix) {
            info!("{}", name);
            file_list.push(SubsystemFile {
                name: name.to_string(),
                path: entry.path().to_path_buf(),
            });
        }
    }

    file_list
}
