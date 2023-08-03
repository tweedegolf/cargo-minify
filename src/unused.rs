use std::{
    collections::HashSet,
    error::Error,
    fmt::{Display, Formatter},
    io::BufReader,
    process::{Command, Stdio},
    str::FromStr,
};

use cargo_metadata::{
    diagnostic::{Diagnostic, DiagnosticSpan},
    Message, Target,
};

use crate::error::Result;

pub fn get_unused(
    targets: &HashSet<Target>,
) -> Result<impl Iterator<Item = UnusedDiagnostic> + '_> {
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
            | UnusedDiagnosticKind::TypeAlias => "is never used",
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

impl Error for NotUnusedDiagnostic {}
