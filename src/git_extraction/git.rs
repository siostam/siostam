use git2::{Branch, BranchType, Remote, Repository, ResetType};
use log::{debug, info, warn};
use std::path::Path;
use std::{fs, thread, time};

/// We only want to get the repo up-to-date without re-cloning every time
/// It deletes the repo folder and reclones it if it can't open it.
pub fn open_and_update_or_clone_repo(url: &str, path: &Path) -> Repository {
    if path.exists() {
        // Try to open the repository then update it
        debug!(
            "Directory {} exists. Trying to open as repository...",
            path.display()
        );
        if let Ok(repo) = Repository::open(path) {
            info!("Repository {} opened.", path.display());
            update_repo(&repo, &path);
            return repo;
        }

        // The path exists and is not valid, this folder must be re-cloned.
        // Remove it then let the clone happen.
        destroy_repo(path);
    }

    // Clone it
    debug!("No repository yet. Cloning at {}", path.display());
    match Repository::clone(url, path) {
        Ok(repo) => {
            info!("Repository cloned at {}.", path.display());
            repo
        }
        Err(e) => panic!("Failed to clone repository: {}", e),
    }
}

/// Fetch data on the `origin` remote for the given repository
pub fn update_repo(repo: &Repository, path: &Path) {
    // Get the link to the remote we want to update.
    // It's always origin in our case. This remote is automatically set when cloning.
    let mut remote: Remote = repo.find_remote("origin").expect("You have no origin?");

    // Woooh, get the updates
    // Maybe TODO display progress to the user
    remote.download(&[], None).expect("Error when downloading");
    remote.disconnect();

    // Display the result to the user
    // Source: https://github.com/rust-lang/git2-rs/blob/master/examples/fetch.rs
    {
        info!("Repository {} updated.", path.display());
        // If there are local objects (we got a thin pack), then tell the user
        // how many objects we saved from having to cross the network.
        let stats = remote.stats();
        if stats.local_objects() > 0 {
            info!(
                "Fetch: received {}/{} objects in {} bytes (used {} local \
                 objects)",
                stats.indexed_objects(),
                stats.total_objects(),
                stats.received_bytes(),
                stats.local_objects()
            );
        } else {
            info!(
                "Fetch: received {}/{} objects in {} bytes",
                stats.indexed_objects(),
                stats.total_objects(),
                stats.received_bytes()
            );
        }
    }
}

/// Make sure we are on the wanted branch with no changes whatsoever
pub fn reset_to_branch(branch_name: &str, repo: &Repository) {
    // We don't want to do any local changes so we can simply use remote branches
    // This allows to find the branch, which is required for the reset thingy
    let branch_name = format!("origin/{}", branch_name);
    let branch: Branch = repo
        .find_branch(branch_name.as_ref(), BranchType::Remote)
        .expect("Branch not found");

    // To do the reset, we need the last commit linked to the branch
    let branch_object = branch.get().peel_to_commit().expect("Commit not found");

    // Reset hard to avoid any remaining changes
    match repo.reset(branch_object.as_object(), ResetType::Hard, None) {
        Ok(()) => info!("Reset to branch {}.", branch_name),
        Err(e) => panic!("Failed to reset at branch {}: {}", branch_name, e),
    }
}

/// Allows to recover from corrupted git repo
pub fn destroy_repo(path: &Path) {
    // If we did not succeed, the repository is possibly broken
    // Then, we remove it
    warn!("Corrupted git repo at {}. Removing it...", path.display());
    fs::remove_dir_all(path).expect("Impossible to remove folder");

    // Wait a moment, just in case
    debug!("Waiting for OS to recover from this terrible loss.");
    thread::sleep(time::Duration::from_secs(1));
}
