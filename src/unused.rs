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
type Y = X;
struct X;
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
        let span = value.spans.into_iter().next().ok_or(NotUnusedDiagnostic)?;

        let (first, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
        match UnusedDiagnosticKind::from_str(first) {
            Ok(kind) => {
                let (mut ident, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
                ident = ident.strip_prefix('`').ok_or(NotUnusedDiagnostic)?;
                ident = ident.strip_suffix('`').ok_or(NotUnusedDiagnostic)?;
                let ident = ident.to_owned();

                if message != "is never used" {
                    return Err(NotUnusedDiagnostic);
                }

                Ok(UnusedDiagnostic { kind, ident, span })
            }
            Err(_) => {
                if first != "unused" {
                    return Err(NotUnusedDiagnostic);
                }

                let (mut kind, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
                kind = kind.strip_suffix(':').ok_or(NotUnusedDiagnostic)?;
                let kind: UnusedDiagnosticKind = kind.parse()?;

                if kind != UnusedDiagnosticKind::Variable {
                    return Err(NotUnusedDiagnostic);
                }

                let (mut ident, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
                ident = ident.strip_prefix('`').ok_or(NotUnusedDiagnostic)?;
                ident = ident.strip_suffix('`').ok_or(NotUnusedDiagnostic)?;
                let ident = ident.to_owned();

                if !message.is_empty() {
                    return Err(NotUnusedDiagnostic);
                }

                Ok(UnusedDiagnostic { kind, ident, span })
            }
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum UnusedDiagnosticKind {
    Constant,
    Function,
    Variable,
}

impl FromStr for UnusedDiagnosticKind {
    type Err = NotUnusedDiagnostic;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "constant" => Ok(UnusedDiagnosticKind::Constant),
            "function" => Ok(UnusedDiagnosticKind::Function),
            "variable" => Ok(UnusedDiagnosticKind::Variable),
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
