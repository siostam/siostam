use log::info;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub struct SubsystemFile {
    pub name: String,
    pub path: PathBuf,
    pub relative_path: String,
    pub repo_name: String,
}

/// List all files in repository with a name ending by the given suffix
pub fn extract_files_from_repo(
    repo_path: &Path,
    repo_name: &str,
    suffix: &str,
) -> Vec<SubsystemFile> {
    let mut file_list: Vec<SubsystemFile> = Vec::new();

    // Recursively list all files
    for entry in WalkDir::new(repo_path) {
        let entry = entry.unwrap();
        let file_name = entry.file_name().to_string_lossy();
        let file_path = entry.path();

        // Ignore all files not matching the pattern specified in the configuration
        if file_name.ends_with(suffix) {
            info!("- {}", file_name);
            file_list.push(SubsystemFile {
                name: file_name.to_string(),
                path: file_path.to_path_buf(),

                // It is always useful to get the source of the data,
                // especially across multiple repositories
                repo_name: repo_name.to_owned(),

                // We prepare the path to be displayed on the front end
                relative_path: file_path
                    .strip_prefix(repo_path)
                    .expect("File path should be a children of the repo_path")
                    .to_str()
                    .map(|path| path.replace("\\", "/"))
                    .unwrap_or(String::from("Corrupted path")),
            });
        }
    }

    file_list
}
