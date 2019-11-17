use git2::Repository;
use log::{debug, info, warn};
use std::path::Path;
use std::{fs, thread, time};

pub fn open_or_clone_repo(url: &str, path: &Path) -> Repository {
    if path.exists() {
        // Try to open the repository
        if let Ok(repo) = Repository::open(path) {
            info!("Opened");
            return repo;
        }

        // If we did not succeed, the repository is possibly broken
        // Then, we remove it
        warn!("Corrupted git repo. Removing...");
        fs::remove_dir_all(path).expect("Impossible to remove folder");

        // Wait a moment, just in case
        debug!("Waiting for OS to recover from this terrible loss.");
        thread::sleep(time::Duration::from_secs(1));
    }

    // Clone it
    debug!("Cloning");
    match Repository::clone(url, path) {
        Ok(repo) => {
            info!("Cloned");
            repo
        }
        Err(e) => panic!("Failed to clone: {}", e),
    }
}
