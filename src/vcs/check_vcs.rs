// Portions of the below code are inspired by/taken from Cargo, https://github.com/rust-lang/cargo/
// Copyright (c) 2016-2021 The Cargo Developers

use std::path::Path;

// Check if we are in an existing repo. We define that to be true if either:
//
// 1. We are in a git repo and the path to the new package is not an ignored
//    path in that repo.
// 2. We are in an HG repo.
pub fn existing_vcs_repo(path: &Path) -> bool {
    in_git_repo(path) || hgrepo_discover(path).is_ok()
}

fn in_git_repo(path: &Path) -> bool {
    std::process::Command::new("git")
        .current_dir(path)
        .arg("check-ignore")
        .arg("-q")
        .arg(".")
        .status()
        .map_or(false, |status| {
            status.code() == Some(1) // status is 0 when ignored, 1 when not ignored and 128 when not a git repo
        })
}

fn hgrepo_discover(path: &Path) -> std::io::Result<()> {
    std::process::Command::new("hg")
        .current_dir(path)
        .arg("--cwd")
        .arg(path)
        .arg("root")
        .output()?;

    Ok(())
}
