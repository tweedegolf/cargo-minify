use std::io;
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
    Error(io::Error),
}

fn check_version_control(path: &Path) -> Status {
    if !check_vcs::existing_vcs_repo(path) {
        return Status::NoVCS;
    }

    let output = match std::process::Command::new("git")
        .current_dir(path)
        .arg("status")
        .arg("--porcelain=v1")
        .output()
    {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            return Status::Error(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "git status failed with exit code {}:\nstdout:\n{}\n\nstderr:\n{}",
                    output.status,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr),
                ),
            ))
        }
        Err(err) => return Status::Error(err),
    };
    let stdout = output.stdout;
    let mut dirty = vec![];
    let mut staged = vec![];
    for line in String::from_utf8_lossy(&stdout).lines() {
        if line.starts_with("M ") || line.starts_with("A ") {
            staged.push(line[2..].to_owned());
        } else if line.starts_with(" M") || line.starts_with("MM") || line.starts_with("??") {
            dirty.push(line[2..].to_owned());
        } else {
            return Status::Error(io::Error::new(
                io::ErrorKind::Other,
                format!("git status returned invalid data: {line:?}"),
            ));
        }
    }

    if dirty.is_empty() && staged.is_empty() {
        Status::Clean
    } else {
        Status::Unclean { dirty, staged }
    }
}
