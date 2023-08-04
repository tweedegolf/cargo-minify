use std::{env, io, io::Write, path::PathBuf};

use gumdrop::Options;

use crate::{diff_format::ColorMode, error::Result};

mod cauterize;
mod diff_format;
mod error;
mod resolver;
mod unused;
mod useless;

const SUBCOMMAND_NAME: &str = "minify";

#[derive(Debug, Options)]
struct MinifyOptions {
    #[options(help = "No output printed to stdout")]
    quiet: bool,

    #[options(help = "Package to minify")]
    package: Vec<String>,
    #[options(no_short, help = "Minify all packages in the workspace")]
    workspace: bool,

    #[options(no_short, help = "Apply changes instead of outputting a diff")]
    apply: bool,

    #[options(help = "Print help message")]
    help: bool,

    #[options(no_short, help = "Coloring: auto, always, never")]
    color: ColorMode,

    #[options(no_short, help = "Path to Cargo.toml")]
    manifest_path: Option<String>,

    #[options(no_short, help = "Fix code even if the working directory is dirty")]
    allow_dirty: bool,

    #[options(no_short, help = "Fix code even if there are staged files in the VCS")]
    allow_staged: bool,

    #[options(no_short, help = "Also operate if no version control system was found")]
    allow_no_vcs: bool,
}

fn main() -> Result<()> {
    // Drop the first actual argument if it is equal to our subcommand
    // (i.e. we are being called via 'cargo')
    let mut args = env::args().peekable();
    args.next();

    if args.peek().map(|s| s.as_str()) == Some(SUBCOMMAND_NAME) {
        args.next();
    }

    execute(&args.collect::<Vec<_>>())?;

    io::stdout().flush()?;

    Ok(())
}

mod vcs;

fn execute(args: &[String]) -> Result<()> {
    let opts = MinifyOptions::parse_args_default(args).expect("internal error");
    let manifest_path = opts.manifest_path.map(PathBuf::from);

    if opts.help {
        println!("{}", MinifyOptions::usage());
    } else {
        let unused = unused::get_unused(manifest_path.as_deref(), &opts.package, opts.workspace)?;
        let changes: Vec<_> = cauterize::process_diagnostics(unused).collect();

        if !opts.quiet {
            if changes.is_empty() {
                println!("no unused code that can be minified")
            } else {
                for change in &changes {
                    diff_format::println(change, opts.color);
                }
            }
        }

        let cargo_root = resolver::get_cargo_metadata(manifest_path.as_deref())?.workspace_root;

        if opts.apply {
            use vcs::Status;
            match vcs::status(&cargo_root) {
                Status::Error(e) => {
                    eprintln!("git problem: {}", e)
                }
                Status::NoVCS if !opts.allow_no_vcs => {
                    eprintln!(
                        "no VCS found for this package and `cargo minify` can potentially perform \
                         destructive changes; if you'd like to suppress this error pass \
                         `--allow-no-vcs`"
                    );
                }
                Status::Unclean { dirty, staged }
                    if !(dirty.is_empty() || opts.allow_dirty)
                        || !(staged.is_empty() || opts.allow_staged) =>
                {
                    eprintln!("working directory contains dirty/staged files:");
                    for file in dirty {
                        eprintln!("\t{} (dirty)", file)
                    }
                    for file in staged {
                        eprintln!("\t{} (staged)", file)
                    }
                    eprintln!(
                        "please fix this or ignore this warning with --allow-dirty and/or \
                         --allow-staged"
                    );
                }
                _ => {
                    // TODO: Remove unwrap
                    cauterize::commit_changes(changes).unwrap();
                }
            }
        } else if !changes.is_empty() {
            println!("run with --apply to apply these changes")
        }
    }

    Ok(())
}
