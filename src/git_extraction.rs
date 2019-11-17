use git2::Repository;
use std::path::Path;
use std::{fs, thread, time};

pub fn open_or_clone_repo(url: &str, path: &Path) -> Repository {
    if path.exists() {
        // Try to open the repository
        if let Ok(repo) = Repository::open(path) {
            println!("Opened");
            return repo;
        }

        // If we did not succeed, the repository is possibly broken
        // Then, we remove it
        println!("Corrupted git repo. Removing...");
        fs::remove_dir_all(path).expect("Impossible to remove folder");

        // Wait a moment, just in case
        thread::sleep(time::Duration::from_secs(1));
    }

    // Clone it
    match Repository::clone(url, path) {
        Ok(repo) => {
            println!("Cloned");
            repo
        }
        Err(e) => panic!("Failed to clone: {}", e),
    }
}
