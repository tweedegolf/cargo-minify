use gumdrop::Options;
use std::{env, io::Write, path::Path};

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
}

/// Function that checks whether the vcs in the given directory is in a "dirty" state
fn vcs_is_dirty(path: impl AsRef<Path>) -> bool {
    true
}

fn main() {
    // Drop the first actual argument if it is equal to our subcommand
    // (i.e. we are being called via 'cargo')
    let args = env::args()
        .enumerate()
        .filter_map(|(n, arg)| (n != 1 || arg != SUBCOMMAND_NAME).then_some(arg))
        .collect::<Vec<_>>();

    let exit_status = execute(&args[1..]);

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
