use std::{
    collections::BTreeMap,
    env, io,
    io::Write,
    path::{Path, PathBuf},
};

use gumdrop::Options;

use crate::error::Result;

mod cauterize;
mod error;
mod resolver;
mod test;
mod unused;

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

    #[options(no_short, help = "Path to Cargo.toml")]
    manifest_path: Option<String>,

    #[options(no_short, help = "Fix code even if the working directory is dirty")]
    allow_dirty: bool,
}

/// Function that checks whether the vcs in the given directory is in a "dirty"
/// state
fn vcs_is_dirty(path: impl AsRef<Path>) -> bool {
    true
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

fn execute(args: &[String]) -> Result<()> {
    let opts = MinifyOptions::parse_args_default(args).expect("internal error");

    if opts.help {
        eprintln!("{}", MinifyOptions::usage());
    } else {
        let manifest_path = opts.manifest_path.map(PathBuf::from);
        let package = &opts.package[..];
        let targets = resolver::get_targets(manifest_path.as_deref(), package, opts.workspace)?;
        // TODO: Remove collect
        let unused: Vec<_> = unused::get_unused(&targets)?.collect();
        let spans = unused.iter().map(|diagnostic| diagnostic.span());

        let changes: BTreeMap<PathBuf, _> = cauterize::process_diagnostics(spans)
            .map(|(file, content)| (PathBuf::from(file), content))
            .collect();

        if !opts.quiet {
            for (file, content) in &changes {
                let content = String::from_utf8(content.clone()).unwrap();
                println!("--------------------------------------------------");
                println!("{:?}", file);
                println!("{content}");
            }
        }

        if opts.apply {
            if opts.allow_dirty || !vcs_is_dirty("") {
                // TODO: Remove unwrap
                cauterize::commit_changes(changes).unwrap();
            } else {
                eprintln!(
                    "working directory is dirty, please commit your changes or ignore this \
                     warning with --allow-dirty"
                );
            }
        }
    }

    Ok(())
}
