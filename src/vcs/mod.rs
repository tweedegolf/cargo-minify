use std::path::Path;

mod check_vcs;

pub fn status(path: impl AsRef<Path>) -> Status {
    check_version_control(path.as_ref())
}

pub enum Status {
    Clean,
    Unclean {
        dirty: Vec<String>,
        staged: Vec<String>,
    },
    NoVCS,
    Error(git2::Error),
}

// Portions of the below code are inspired by/taken from Cargo, https://github.com/rust-lang/cargo/
// Copyright (c) 2016-2021 The Cargo Developers

fn check_version_control(path: &Path) -> Status {
    if !check_vcs::existing_vcs_repo(path) {
        return Status::NoVCS;
    }

    let mut dirty = Vec::new();
    let mut staged = Vec::new();
    if let Ok(repo) = git2::Repository::discover(path) {
        let mut repo_opts = git2::StatusOptions::new();
        repo_opts.include_ignored(false);
        repo_opts.include_untracked(true);
        let statuses = match repo.statuses(Some(&mut repo_opts)) {
            Ok(value) => value,
            Err(error) => return Status::Error(error),
        };
        for status in statuses.iter() {
            if let Some(path) = status.path() {
                match status.status() {
                    git2::Status::CURRENT => (),
                    git2::Status::INDEX_NEW
                    | git2::Status::INDEX_MODIFIED
                    | git2::Status::INDEX_DELETED
                    | git2::Status::INDEX_RENAMED
                    | git2::Status::INDEX_TYPECHANGE => staged.push(path.to_string()),
                    _ => dirty.push(path.to_string()),
                };
            }
        }
    }

    if dirty.is_empty() && staged.is_empty() {
        Status::Clean
    } else {
        Status::Unclean { dirty, staged }
    }
}
