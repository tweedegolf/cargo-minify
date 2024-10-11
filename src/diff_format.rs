use std::str::FromStr;

use thiserror::Error;

use crate::cauterize::Change;

const BEFORE_CONTEXT: isize = 3;
const AFTER_CONTEXT: isize = 3;

pub fn println(change: &Change, color_mode: ColorMode) {
    let text = format!("#\n#\tshowing diff for {:?}:\n#", change.file_name());
    if color_mode.enabled() {
        println!("\x1b[90m{text}\x1b[0m");
    } else {
        println!("{text}")
    }

    let left = String::from_utf8_lossy(change.original_content());
    let right = String::from_utf8_lossy(change.proposed_content());

    let diff = diff::lines(&left, &right);

    let mut included = Vec::new();

    let mut last_change: isize = -AFTER_CONTEXT - 1;
    let mut last_insert: isize = -AFTER_CONTEXT - 1;
    for index in 0..diff.len() as isize {
        if has_changed(&diff[index as usize]) {
            if last_insert < index - BEFORE_CONTEXT - 1 {
                included.push(DiffLine::Ellipsis);
            }
            for index in (index - BEFORE_CONTEXT).max(last_insert + 1).max(0)..index {
                included.push(DiffLine::Context(get_line(&diff[index as usize])));
            }
            included.push(DiffLine::Diff(diff[index as usize].clone()));
            last_insert = index;
            last_change = index;
        } else if index - last_change <= AFTER_CONTEXT {
            included.push(DiffLine::Context(get_line(&diff[index as usize])));
            last_insert = index;
        }
    }
    if last_insert < diff.len() as isize - 1 {
        included.push(DiffLine::Ellipsis);
    }

    for line in included {
        let (symbol, color, line) = match line {
            DiffLine::Diff(diff::Result::Left(line)) => ('-', "91", line),
            DiffLine::Diff(diff::Result::Right(line)) => ('+', "92", line),
            DiffLine::Diff(diff::Result::Both(_, _)) => unreachable!(),
            DiffLine::Context(line) => (' ', "39", line),
            DiffLine::Ellipsis => ('#', "90", "..."),
        };

        let format = format!("{symbol}\t{line}");

        if color_mode.enabled() {
            println!("\x1b[{color}m{format}\x1b[0m");
        } else {
            println!("{format}");
        }
    }
}

fn has_changed(diff: &diff::Result<&str>) -> bool {
    match diff {
        diff::Result::Left(_) | diff::Result::Right(_) => true,
        diff::Result::Both(_, _) => false,
    }
}

fn get_line<'a>(diff: &diff::Result<&'a str>) -> &'a str {
    match diff {
        diff::Result::Left(line) | diff::Result::Right(line) => line,
        diff::Result::Both(line, _) => line,
    }
}

enum DiffLine<T> {
    Diff(diff::Result<T>),
    Context(T),
    Ellipsis,
}

#[derive(Copy, Clone, Debug, Default)]
pub enum ColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

impl ColorMode {
    pub fn enabled(&self) -> bool {
        match self {
            // TODO: Improve
            ColorMode::Auto => true,
            ColorMode::Always => true,
            ColorMode::Never => false,
        }
    }
}

impl FromStr for ColorMode {
    type Err = UnsupportedPrintColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "auto" => Ok(ColorMode::Auto),
            "always" => Ok(ColorMode::Always),
            "never" => Ok(ColorMode::Never),
            _ => Err(UnsupportedPrintColor),
        }
    }
}

#[derive(Debug, Error)]
#[error("unsupported color mode, pick any of: auto, always, never")]
pub struct UnsupportedPrintColor;
