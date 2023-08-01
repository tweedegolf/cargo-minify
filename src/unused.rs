use std::{
    error::Error,
    fmt::{Display, Formatter},
    str::FromStr,
};

use cargo_metadata::diagnostic::{Diagnostic, DiagnosticSpan};

#[derive(Debug)]
pub struct UnusedDiagnostic {
    kind: UnusedDiagnosticKind,
    ident: String,
    span: DiagnosticSpan,
}

impl TryFrom<Diagnostic> for UnusedDiagnostic {
    type Error = NotUnusedDiagnostic;

    fn try_from(value: Diagnostic) -> Result<Self, Self::Error> {
        let message = value.message;

        let (kind, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
        let kind = UnusedDiagnosticKind::from_str(kind)?;

        let (mut ident, message) = message.split_once(' ').ok_or(NotUnusedDiagnostic)?;
        ident = ident.strip_prefix('`').ok_or(NotUnusedDiagnostic)?;
        ident = ident.strip_suffix('`').ok_or(NotUnusedDiagnostic)?;
        let ident = ident.to_owned();

        if message != "is never used" {
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
}

impl FromStr for UnusedDiagnosticKind {
    type Err = NotUnusedDiagnostic;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "constant" => Ok(UnusedDiagnosticKind::Constant),
            "function" => Ok(UnusedDiagnosticKind::Function),
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
