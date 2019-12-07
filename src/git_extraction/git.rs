use crate::error::CustomError;
use git2::build::RepoBuilder;
use git2::{
    AutotagOption, Branch, BranchType, Cred, FetchOptions, Remote, RemoteCallbacks, Repository,
    ResetType,
};
use log::{debug, info, log_enabled, trace, warn, Level};
use std::path::Path;
use std::{env, fs, thread, time};

/// We only want to get the repo up-to-date without re-cloning every time
/// It deletes the repo folder and re-clones it if it can't open it.
pub fn open_and_update_or_clone_repo(
    url: &str,
    path: &Path,
    callbacks: RemoteCallbacks,
) -> Result<Repository, CustomError> {
    if path.exists() {
        // Try to open the repository then update it
        debug!(
            "Directory {} exists. Trying to open as repository...",
            path.display()
        );
        if let Ok(repo) = Repository::open(path) {
            info!("Repository {} opened.", path.display());
            update_repo(&repo, &path, callbacks)?;
            return Ok(repo);
        }

        // The path exists and is not valid, this folder must be re-cloned.
        // Remove it then let the clone happen.
        destroy_repo(path);
    }

    // Clone it
    debug!("No repository yet. Cloning {} at {}", url, path.display());
    let mut builder = RepoBuilder::new();
    let mut fetch_options = FetchOptions::new();

    fetch_options.remote_callbacks(callbacks);
    builder.fetch_options(fetch_options);

    match builder.clone(url, path) {
        Ok(repo) => {
            info!("Repository cloned at {}.", path.display());
            Ok(repo)
        }
        Err(e) => Err(CustomError::new(format!(
            "Failed to clone repository: {}",
            e
        ))),
    }
}

/// Create an object with the callbacks to handle self_certs and auth
pub fn provide_callbacks(callbacks: &mut RemoteCallbacks) {
    // Always bypass because we are accessing in read-only
    // TODO Check if this is really okay
    callbacks.certificate_check(|_cert, _str| true);

    // This callback gets called for each remote-tracking branch that gets
    // updated. The message we output depends on whether it's a new one or an
    // update.
    callbacks.update_tips(|refname, a, b| {
        if a.is_zero() {
            info!("[new]     {:20} {}", b, refname);
        } else {
            info!("[updated] {:10}..{:10} {}", a, b, refname);
        }
        true
    });

    let mut tries = 0;

    // Authenticate by ssh key if they are provided
    // Source: https://wapl.es/rust/2017/10/06/git2-rs-cloning-private-github-repos.html
    callbacks.credentials(move |_url, user_from_url, cred| {
        tries = tries + 1;

        // Do not attempt to many times
        if tries > 3 {
            return Err(git2::Error::from_str("3 fails. Stop it before killing the server."))
        }

        // Throttle tries 2 and 3 to avoid killing the server
        if tries > 1 {
            thread::sleep(time::Duration::from_secs(1));
        }

        if log_enabled!(Level::Debug) {
            trace!("url={}, user={}, is_user_pass_plaintext={:?}, is_ssh_key={:?}, is_ssh_memory={:?}, is_ssh_custom={:?}, is_default={:?}, is_ssh_interactive={:?}, is_username={}",
                     _url,
                     user_from_url.unwrap_or("--"),
                     cred.is_user_pass_plaintext(),
                     cred.is_ssh_key(),
                     cred.is_ssh_memory(),
                     cred.is_ssh_custom(),
                     cred.is_default(),
                     cred.is_ssh_interactive(),
                     cred.is_username());
        }

        if cred.contains(git2::CredentialType::USERNAME) {
            git2::Cred::username("git")
        }
        else if cred.contains(git2::CredentialType::SSH_KEY) {
            // TODO Fix SSH authentication. Completely broken at the time
            let public_key = env::var("SUBSYSTEM_MAPPER_GIT_SSH_PUBLIC_KEY").ok();
            let private_key = env::var("SUBSYSTEM_MAPPER_GIT_SSH_PRIVATE_KEY").expect("private_key is mandatory in this case");
            let passphrase = env::var("SUBSYSTEM_MAPPER_GIT_SSH_PASSPHRASE").ok();

            // The actual ssh credentials
            Ok(Cred::ssh_key(
                "git",
                public_key
                        .as_ref()
                        .map(|x| Path::new(&**x)),
                Path::new(private_key.as_str()),
                passphrase.as_ref().map(|x|&**x)
            ).expect("Could not create credentials object"))
        }
        else if cred.contains(git2::CredentialType::USER_PASS_PLAINTEXT){
            // Transform Option<String> in Option<&str>
            // Source: https://stackoverflow.com/questions/31233938/converting-from-optionstring-to-optionstr
            let username = env::var("SUBSYSTEM_MAPPER_GIT_HTTPS_USERNAME").expect("Username is mandatory in this case");
            let password = env::var("SUBSYSTEM_MAPPER_GIT_HTTPS_PASSWORD").expect("Password is mandatory in this case");

            Ok(Cred::userpass_plaintext(
                username.as_str(),
                    password.as_str()
                ).expect("Could not create credentials object"))
        }
        else {
            Err(git2::Error::from_str("Authentication method not supported"))
        }
    });
}

/// Fetch data on the `origin` remote for the given repository
pub fn update_repo(
    repo: &Repository,
    path: &Path,
    callbacks: RemoteCallbacks,
) -> Result<(), CustomError> {
    // Many instructions and comments are from the git2-rs fetch example
    // Source: https://github.com/rust-lang/git2-rs/blob/master/examples/fetch.rs

    // Get the link to the remote we want to update.
    // It's always origin in our case. This remote is automatically set when cloning.
    let mut remote: Remote = repo.find_remote("origin").expect("You have no origin?");

    // Create an option to provide callbacks
    let mut fetch_options = FetchOptions::default();
    fetch_options.remote_callbacks(callbacks);

    // Woooh, get the updates
    // Maybe TODO display progress to the user
    remote
        .download(&[], Some(&mut fetch_options))
        .map_err(|err| CustomError::new(format!("Error when updating tips: {}", err)))?;
    remote.disconnect();

    // Update the references in the remote's namespace to point to the right
    // commits. This may be needed even if there was no packfile to download,
    // which can happen e.g. when the branches have been changed but all the
    // needed objects are available locally.
    remote
        .update_tips(None, true, AutotagOption::Unspecified, None)
        .map_err(|err| CustomError::new(format!("Error when updating tips: {}", err)))?;

    // Display the result to the user
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

    Ok(())
}

/// Make sure we are on the wanted branch with no changes whatsoever
pub fn reset_to_branch(
    branch_name: &str,
    repo: &Repository,
    repo_name: &str,
) -> Result<(), CustomError> {
    // We don't want to do any local changes so we can simply use remote branches
    // This allows to find the branch, which is required for the reset thingy
    let branch_name = format!("origin/{}", branch_name);
    let branch: Branch = repo
        .find_branch(branch_name.as_ref(), BranchType::Remote)
        .map_err(|e| {
            CustomError::new(format!(
                "Failed to find branch for repo {}: {}",
                repo_name, e
            ))
        })?;

    // To do the reset, we need the last commit linked to the branch
    let branch_object = branch.get().peel_to_commit().expect("Commit not found");

    // Reset hard to avoid any remaining changes
    repo.reset(branch_object.as_object(), ResetType::Hard, None)
        .map_err(|e| {
            CustomError::new(format!(
                "Failed to reset {} at branch {}: {}",
                repo_name, branch_name, e
            ))
        })?;

    // Display a message with details for further analysis
    info!(
        "Reset to branch {} with last change by {}",
        branch_name,
        branch_object.committer().name().unwrap_or("Unknown"),
    );
    info!(
        "{} {}",
        branch_object.id(),
        branch_object.summary().unwrap_or("no message")
    );

    Ok(())
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
