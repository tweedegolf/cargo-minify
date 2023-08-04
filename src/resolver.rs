//! This module contains the functionality necessary to find which packages the
//! user wants to minify.

// Portions of the below code are inspired by/taken from Rustfmt, https://github.com/rust-lang/rustfmt
// Copyright (c) 2016-2021 The Rust Project Developers

use std::{
    collections::{BTreeSet, HashSet},
    env, io,
    path::{Path, PathBuf},
};

use cargo_metadata::Target;

use crate::{error::Result, CrateResolutionOptions};

pub fn get_targets(
    manifest_path: Option<&Path>,
    crate_resolution: &CrateResolutionOptions,
) -> Result<HashSet<Target>> {
    let mut targets = HashSet::new();

    match crate_resolution {
        CrateResolutionOptions::Root => root_targets(manifest_path, &mut targets)?,
        CrateResolutionOptions::Workspace { exclude } => {
            workspace_targets(manifest_path, *exclude, &mut targets, &mut BTreeSet::new())?
        }
        CrateResolutionOptions::Package { packages } => {
            package_targets(manifest_path, *packages, &mut targets)?
        }
    }

    eprintln!("crate resolution found no targets");

    Ok(targets)
}

fn root_targets(manifest_path: Option<&Path>, targets: &mut HashSet<Target>) -> Result<()> {
    let metadata = get_cargo_metadata(manifest_path)?;
    let workspace_root_path = PathBuf::from(&metadata.workspace_root).canonicalize()?;
    let (in_workspace_root, current_dir_manifest) = if let Some(target_manifest) = manifest_path {
        (
            workspace_root_path == target_manifest,
            target_manifest.canonicalize()?,
        )
    } else {
        let current_dir = env::current_dir()?.canonicalize()?;
        (
            workspace_root_path == current_dir,
            current_dir.join("Cargo.toml"),
        )
    };

    let package_targets = match metadata.packages.len() {
        1 => metadata.packages.into_iter().next().unwrap().targets,
        _ => metadata
            .packages
            .into_iter()
            .filter(|p| {
                in_workspace_root
                    || PathBuf::from(&p.manifest_path)
                        .canonicalize()
                        .unwrap_or_default()
                        == current_dir_manifest
            })
            .flat_map(|p| p.targets)
            .collect(),
    };

    for target in package_targets {
        targets.insert(target);
    }

    Ok(())
}

fn workspace_targets(
    manifest_path: Option<&Path>,
    exclude: &[String],
    targets: &mut HashSet<Target>,
    visited: &mut BTreeSet<String>,
) -> Result<()> {
    let metadata = get_cargo_metadata(manifest_path)?;
    for package in &metadata.packages {
        if !exclude
            .iter()
            .any(|name| glob_match::glob_match(&package.name, name))
        {
            for target in &package.targets {
                targets.insert(target.clone());
            }

            for dependency in &package.dependencies {
                if dependency.path.is_none() || visited.contains(&dependency.name) {
                    continue;
                }

                let manifest_path =
                    PathBuf::from(dependency.path.as_ref().unwrap()).join("Cargo.toml");
                if manifest_path.exists()
                    && !metadata
                        .packages
                        .iter()
                        .any(|p| p.manifest_path.eq(&manifest_path))
                {
                    visited.insert(dependency.name.to_owned());
                    workspace_targets(Some(&manifest_path), exclude, targets, visited)?;
                }
            }
        }
    }

    Ok(())
}

fn package_targets(
    manifest_path: Option<&Path>,
    hitlist: &[String],
    targets: &mut HashSet<Target>,
) -> Result<()> {
    let metadata = get_cargo_metadata(manifest_path)?;
    let mut workspace_hitlist: BTreeSet<&String> = BTreeSet::from_iter(hitlist);

    for package in metadata.packages {
        if workspace_hitlist.remove(&package.name) {
            for target in package.targets {
                targets.insert(target);
            }
        }
    }

    if workspace_hitlist.is_empty() {
        Ok(())
    } else {
        let package = workspace_hitlist.iter().next().unwrap();
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("package `{}` is not a member of the workspace", package),
        )
        .into())
    }
}

pub fn get_cargo_metadata(manifest_path: Option<&Path>) -> Result<cargo_metadata::Metadata> {
    let mut cmd = cargo_metadata::MetadataCommand::new();
    cmd.no_deps();
    if let Some(manifest_path) = manifest_path {
        cmd.manifest_path(manifest_path);
    }
    cmd.other_options(vec![String::from("--offline")]);

    match cmd.exec() {
        Ok(metadata) => Ok(metadata),
        Err(_) => {
            cmd.other_options(vec![]);
            match cmd.exec() {
                Ok(metadata) => Ok(metadata),
                Err(error) => Err(io::Error::new(io::ErrorKind::Other, error.to_string()).into()),
            }
        }
    }
}
