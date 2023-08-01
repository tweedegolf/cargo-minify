use gumdrop::Options;
use std::{env, io::Write, path::Path};

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
    manifest_path: bool,
    //TODO: probably also specify which file to minify
}

/// Function that checks whether the vcs in the given directory is in a "dirty" state
fn vcs_is_dirty(path: impl AsRef<Path>) -> bool {
    true
}

fn main() {
    // Drop the first actual argument if it is equal to our subcommand
    // (i.e. we are being called via 'cargo')
    let mut args = env::args().peekable();
    args.next();

    if args.peek().map(|s| -> &str { s }) == Some(SUBCOMMAND_NAME) {
        args.next();
    }

    let exit_status = execute(&args.collect::<Vec<_>>());

    std::io::stdout().flush().unwrap();
    std::process::exit(exit_status);
}

const SUCCESS: i32 = 0;
const FAILURE: i32 = 1;

fn execute(args: &[String]) -> i32 {
    let opts = MinifyOptions::parse_args_default(args).expect("internal error");
    if opts.help {
        eprintln!("{}", MinifyOptions::usage());
        return SUCCESS;
    } else {
        todo!()
    }

    SUCCESS
}
