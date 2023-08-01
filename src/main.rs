use std::{
    collections::HashSet,
    env, io,
    io::{BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use cargo_metadata::{Message, Target};
use gumdrop::Options;

use crate::{error::Result, unused::UnusedDiagnostic};

mod error;
mod resolver;
mod unused;

mod cauterize;

const SUBCOMMAND_NAME: &str = "minify";

#[derive(Debug, Options)]
struct MinifyOptions {
    #[options(help = "print help message")]
    help: bool,

    #[options(no_short, help = "actually apply changes instead of outputting a diff")]
    fix: bool,

    #[options(
        no_short,
        help = "really apply changes even if this is NOT recommended and will lead to loss of code"
    )]
    yes_damnit: bool,

    #[options(no_short, help = "path to Cargo.toml")]
    manifest_path: Option<String>,

    #[options(help = "package to minify")]
    package: Vec<String>,
    #[options(no_short, help = "minify all packages in the workspace")]
    workspace: bool,
    #[options(help = "exclude packages from the build")]
    exclude: Vec<String>,
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

const SUCCESS: i32 = 0;
const FAILURE: i32 = 1;

fn execute(args: &[String]) -> Result<()> {
    let opts = MinifyOptions::parse_args_default(args).expect("internal error");

    if opts.help {
        eprintln!("{}", MinifyOptions::usage());
    } else {
        let manifest_path = opts.manifest_path.map(PathBuf::from);
        let package = &opts.package[..];
        let exclude = &opts.package[..];
        let targets =
            resolver::get_targets(manifest_path.as_deref(), package, opts.workspace, exclude)?;

        for unused in unused(&targets)? {
            println!("{:#?}", unused);
        }
    }

    Ok(())
}

fn unused(targets: &HashSet<Target>) -> Result<impl Iterator<Item = UnusedDiagnostic> + '_> {
    let mut command = Command::new("cargo");
    command.args(["check", "--message-format", "json"]);

    let mut child = command.stdout(Stdio::piped()).spawn()?;
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let unused = Message::parse_stream(reader)
        .flatten()
        .filter_map(|message| {
            if let Message::CompilerMessage(message) = message {
                Some(message)
            } else {
                None
            }
        })
        .filter(|message| targets.contains(&message.target))
        .map(|message| message.message)
        .filter_map(|diagnostic| UnusedDiagnostic::try_from(diagnostic).ok());

    Ok(unused)
}
