use std::{
    fmt::{Display, Formatter},
    io::BufReader,
    path::Path,
    process::{Command, Stdio},
    str::FromStr,
};

use cargo_metadata::{
    diagnostic::{Diagnostic, DiagnosticSpan},
    Message,
};

use crate::{error::Result, resolver, CrateResolutionOptions, FileResolutionOptions};

pub fn get_unused<'a>(
    manifest_path: Option<&Path>,
    crate_resolution: &CrateResolutionOptions,
    file_resolution: &'a FileResolutionOptions,
) -> Result<impl Iterator<Item = UnusedDiagnostic> + 'a> {
    let mut command = Command::new("cargo");

    command.args(["check", "--message-format", "json"]);

    match crate_resolution {
        CrateResolutionOptions::Root => {}
        CrateResolutionOptions::Workspace { exclude } => {
            command.arg("--workspace");

            for package in *exclude {
                command.args(["--exclude", package]);
            }
        }
        CrateResolutionOptions::Package { packages } => {
            for package in *packages {
                command.args(["-p", package]);
            }
        }
    }

    let mut child = command.stdout(Stdio::piped()).spawn()?;
    let stdout = child.stdout.take().unwrap();
    let reader = BufReader::new(stdout);

    let targets = resolver::get_targets(manifest_path, crate_resolution)?;

    let unused = Message::parse_stream(reader)
        .flatten()
        .filter_map(|message| {
            if let Message::CompilerMessage(message) = message {
                Some(message)
            } else {
                None
            }
        })
        .filter(move |message| targets.contains(&message.target))
        .map(|message| message.message)
        .filter_map(|diagnostic| UnusedDiagnostic::try_from(diagnostic).ok())
        .filter(|diagnostic| file_resolution.is_included(&diagnostic.span.file_name));

    Ok(unused)
}

#[derive(Debug)]
pub struct UnusedDiagnostic {
    kind: UnusedDiagnosticKind,
    ident: String,
    span: DiagnosticSpan,
}

impl UnusedDiagnostic {
    pub fn span(&self) -> &DiagnosticSpan {
        &self.span
    }
}

impl TryFrom<Diagnostic> for UnusedDiagnostic {
    type Error = NotUnusedDiagnostic;

    fn try_from(value: Diagnostic) -> Result<Self, Self::Error> {
        let message = value.message;

        let (kind, mut message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
        let kind = UnusedDiagnosticKind::from_str(kind)?;

        message = match kind {
            UnusedDiagnosticKind::Constant
            | UnusedDiagnosticKind::Function
            | UnusedDiagnosticKind::Struct
            | UnusedDiagnosticKind::Enum
            | UnusedDiagnosticKind::Union => message,
            UnusedDiagnosticKind::TypeAlias => {
                let (alias, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;

                if alias != "alias" {
                    return Err(NotUnusedDiagnostic);
                }

                message
            }
            UnusedDiagnosticKind::AssociatedFunction => {
                let (alias, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;

                if alias != "function" {
                    return Err(NotUnusedDiagnostic);
                }

                message
            }
        };

        let (mut ident, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
        ident = ident.strip_prefix('`').ok_or(NotUnusedDiagnostic)?;
        ident = ident.strip_suffix('`').ok_or(NotUnusedDiagnostic)?;
        let ident = ident.to_owned();

        let suffix = match kind {
            UnusedDiagnosticKind::Constant
            | UnusedDiagnosticKind::Function
            | UnusedDiagnosticKind::Enum
            | UnusedDiagnosticKind::Union
            | UnusedDiagnosticKind::TypeAlias
            | UnusedDiagnosticKind::AssociatedFunction => "is never used",
            UnusedDiagnosticKind::Struct => "is never constructed",
        };

        if message != suffix {
            return Err(NotUnusedDiagnostic);
        }

        let span = value.spans.into_iter().next().ok_or(NotUnusedDiagnostic)?;

        Ok(UnusedDiagnostic { kind, ident, span })
    }
}

#[derive(Debug)]
pub enum UnusedDiagnosticKind {
    Constant,
    Function,
    Struct,
    Enum,
    Union,
    TypeAlias,
    AssociatedFunction,
}

impl FromStr for UnusedDiagnosticKind {
    type Err = NotUnusedDiagnostic;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "constant" => Ok(UnusedDiagnosticKind::Constant),
            "function" => Ok(UnusedDiagnosticKind::Function),
            "struct" => Ok(UnusedDiagnosticKind::Struct),
            "enum" => Ok(UnusedDiagnosticKind::Enum),
            "union" => Ok(UnusedDiagnosticKind::Union),
            "type" => Ok(UnusedDiagnosticKind::TypeAlias),
            "associated" => Ok(UnusedDiagnosticKind::AssociatedFunction),
            _ => Err(NotUnusedDiagnostic),
        }
    }
}

#[derive(Debug)]
pub struct NotUnusedDiagnostic;

impl Display for NotUnusedDiagnostic {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "not an unused-diagnostic")
    }
}

impl std::error::Error for NotUnusedDiagnostic {}
