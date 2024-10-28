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
    kinds: &'a [UnusedDiagnosticKind],
) -> Result<impl Iterator<Item = UnusedDiagnostic> + 'a> {
    let mut command = Command::new("cargo");

    command.args(["check", "--all-targets", "--quiet", "--message-format", "json"]);

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
        // Ignore unused warnings originating from macro expansions
        .filter(|diagnostic| diagnostic.span.expansion.is_none())
        .filter(|diagnostic| kinds.is_empty() || kinds.contains(&diagnostic.kind))
        .filter(|diagnostic| file_resolution.is_included(&diagnostic.span.file_name));

    Ok(unused)
}

#[derive(Debug)]
pub struct UnusedDiagnostic {
    pub kind: UnusedDiagnosticKind,
    pub ident: String,
    pub span: DiagnosticSpan,
}

impl TryFrom<Diagnostic> for UnusedDiagnostic {
    type Error = NotUnusedDiagnostic;

    fn try_from(value: Diagnostic) -> Result<Self, Self::Error> {
        let message = value.message;

        let (first, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
        match UnusedDiagnosticKind::from_str(first) {
            Ok(kind) => {
                let message = match kind {
                    UnusedDiagnosticKind::Constant
                    | UnusedDiagnosticKind::Static
                    | UnusedDiagnosticKind::Function
                    | UnusedDiagnosticKind::Struct
                    | UnusedDiagnosticKind::Enum
                    | UnusedDiagnosticKind::Union => message,
                    UnusedDiagnosticKind::TypeAlias => {
                        let (alias, message) =
                            message.split_once(' ').ok_or(NotUnusedDiagnostic)?;

                        if alias != "alias" {
                            return Err(NotUnusedDiagnostic);
                        }

                        message
                    }
                    UnusedDiagnosticKind::AssociatedFunction => {
                        let (function, message) =
                            message.split_once(' ').ok_or(NotUnusedDiagnostic)?;

                        if function != "function" {
                            return Err(NotUnusedDiagnostic);
                        }

                        message
                    }
                    UnusedDiagnosticKind::MacroDefinition => return Err(NotUnusedDiagnostic),
                };

                let (mut ident, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
                ident = ident.strip_prefix('`').ok_or(NotUnusedDiagnostic)?;
                ident = ident.strip_suffix('`').ok_or(NotUnusedDiagnostic)?;
                let ident = ident.to_owned();

                let suffix = match kind {
                    UnusedDiagnosticKind::Constant
                    | UnusedDiagnosticKind::Static
                    | UnusedDiagnosticKind::Function
                    | UnusedDiagnosticKind::Enum
                    | UnusedDiagnosticKind::Union
                    | UnusedDiagnosticKind::TypeAlias
                    | UnusedDiagnosticKind::AssociatedFunction => "is never used",
                    UnusedDiagnosticKind::Struct => "is never constructed",
                    UnusedDiagnosticKind::MacroDefinition => return Err(NotUnusedDiagnostic),
                };

                if message != suffix {
                    return Err(NotUnusedDiagnostic);
                }

                let span = value.spans.into_iter().next().ok_or(NotUnusedDiagnostic)?;

                Ok(UnusedDiagnostic { kind, ident, span })
            }
            Err(_) => {
                if first != "unused" {
                    return Err(NotUnusedDiagnostic);
                }

                let (mut kind, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
                kind = kind.strip_suffix(':').unwrap_or(kind);
                let kind: UnusedDiagnosticKind = kind.parse()?;

                let message = match kind {
                    UnusedDiagnosticKind::Constant
                    | UnusedDiagnosticKind::Static
                    | UnusedDiagnosticKind::Function
                    | UnusedDiagnosticKind::Struct
                    | UnusedDiagnosticKind::Enum
                    | UnusedDiagnosticKind::Union
                    | UnusedDiagnosticKind::TypeAlias
                    | UnusedDiagnosticKind::AssociatedFunction => return Err(NotUnusedDiagnostic),
                    UnusedDiagnosticKind::MacroDefinition => {
                        let (definition, message) =
                            message.split_once(' ').ok_or(NotUnusedDiagnostic)?;

                        if definition != "definition:" {
                            return Err(NotUnusedDiagnostic);
                        }

                        message
                    }
                };

                let mut split = message.splitn(2, ' ');
                let mut ident = split.next().unwrap();
                let message = split.next().unwrap_or_default();

                ident = ident.strip_prefix('`').ok_or(NotUnusedDiagnostic)?;
                ident = ident.strip_suffix('`').ok_or(NotUnusedDiagnostic)?;
                let ident = ident.to_owned();

                if !message.is_empty() {
                    return Err(NotUnusedDiagnostic);
                }

                let span = value.spans.into_iter().next().ok_or(NotUnusedDiagnostic)?;

                Ok(UnusedDiagnostic { kind, ident, span })
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum UnusedDiagnosticKind {
    Constant,
    Static,
    Function,
    Struct,
    Enum,
    Union,
    TypeAlias,
    AssociatedFunction,
    MacroDefinition,
}

impl FromStr for UnusedDiagnosticKind {
    type Err = NotUnusedDiagnostic;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s
            .chars()
            .filter(|c| c.is_alphanumeric())
            .map(|c| c.to_ascii_lowercase())
            .collect::<String>()
            .as_str()
        {
            "constant" => Ok(UnusedDiagnosticKind::Constant),
            "static" => Ok(UnusedDiagnosticKind::Static),
            "function" => Ok(UnusedDiagnosticKind::Function),
            "struct" => Ok(UnusedDiagnosticKind::Struct),
            "enum" => Ok(UnusedDiagnosticKind::Enum),
            "union" => Ok(UnusedDiagnosticKind::Union),
            "type" | "typealias" => Ok(UnusedDiagnosticKind::TypeAlias),
            "associated" | "associatedfunction" => Ok(UnusedDiagnosticKind::AssociatedFunction),
            "macro" | "macrodefinition" => Ok(UnusedDiagnosticKind::MacroDefinition),
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
