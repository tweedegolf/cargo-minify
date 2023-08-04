use std::{
    ops::Range,
    path::{Path, PathBuf},
};

use crate::unused::UnusedDiagnostic;

const SPACE: u8 = b' ';
const NEWLINE: u8 = b'\n';

pub struct Change {
    file_name: PathBuf,
    original_content: Vec<u8>,
    proposed_content: Vec<u8>,
}

impl Change {
    pub fn file_name(&self) -> &Path {
        &self.file_name
    }

    pub fn original_content(&self) -> &[u8] {
        &self.original_content
    }

    pub fn proposed_content(&self) -> &[u8] {
        &self.proposed_content
    }
}

/// Turns a list of "locations of identifiers" into a list of "chunk
/// BUGS: this is not safe for use on macros
fn rust_identifiers_to_definitions<'a>(
    src: &'a [u8],
    locations: impl IntoIterator<Item = usize> + 'a,
) -> impl Iterator<Item = Range<usize>> + 'a {
    locations.into_iter().map(|pos| {
        let prev = src[..pos]
            .iter()
            .rposition(|x| b";}{".contains(x))
            .map(|i| {
                (i + 1..pos)
                    .take_while(|&k| src[k].is_ascii_whitespace())
                    .last()
                    .map(|j| j + 1)
                    .unwrap_or(i + 1)
            })
            .unwrap_or(0);
        let next = src[pos..]
            .iter()
            .position(|x| b";{".contains(x))
            .map(|i| pos + i)
            .map(|mut i| {
                // find matching '}' for a '{'
                let mut level = 0;
                let mut in_quote = None;
                loop {
                    match src[i] {
                        x if Some(x) == in_quote => in_quote = None,
                        _ if in_quote.is_some() => {}
                        b'{' => level += 1,
                        b'}' => level -= 1,
                        b'"' => in_quote = Some(b'"'),
                        b'\'' => in_quote = Some(b'\''),
                        _ => {}
                    }
                    i += 1;

                    if i == src.len() {
                        return i;
                    }
                    if level == 0 {
                        break;
                    }
                }

                src[i..]
                    .iter()
                    .position(|c| *c != SPACE)
                    .map(|k| i + k)
                    .map(|pos| if src[pos] == NEWLINE { pos + 1 } else { pos })
                    .unwrap_or(src.len())
            })
            .unwrap_or(src.len());

        prev..next
    })
}

/// Deletes a list-of-positions-of-identifiers from a bytearray that is valid
/// rust code BUGS: if the position is in the body of a function, it will try to
/// delete identifiers there ...  probably?
pub fn rust_delete(src: &[u8], locations: impl IntoIterator<Item = usize>) -> Vec<u8> {
    let chunks_to_delete = rust_identifiers_to_definitions(src, locations).collect::<Vec<_>>();
    src.iter()
        .enumerate()
        .filter_map(|(i, &byte)| {
            if chunks_to_delete.iter().any(|range| range.contains(&i)) {
                None
            } else {
                Some(byte)
            }
        })
        .collect()
}

/// Processes a list of file+list-of-edits into an iterator of
/// filenames+proposed new contents
fn process_files<Iter: IntoIterator<Item = usize>>(
    diagnostics: impl IntoIterator<Item = (PathBuf, Iter)>,
) -> impl Iterator<Item = Change> {
    diagnostics
        .into_iter()
        .filter_map(|(file_name, byte_locations)| {
            let original_content = std::fs::read(&file_name).ok()?;
            let proposed_content = rust_delete(&original_content, byte_locations);

            let change = Change {
                file_name,
                original_content,
                proposed_content,
            };

            Some(change)
        })
}

/// Process a list of UnusedDiagnostics into an iterator of filenames+proposed
/// contents BUGS: this does not check that the diagnostic is a "unused
/// diagnostic"
pub fn process_diagnostics(
    diagnostics: impl IntoIterator<Item = UnusedDiagnostic>,
) -> impl Iterator<Item = Change> {
    process_files(
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                let span = diagnostic.span();
                let path = PathBuf::from(&span.file_name);
                let start = span.byte_start as usize;
                (path, start)
            })
            .collect::<multimap::MultiMap<_, _>>()
            .into_iter(),
    )
}

/// DANGER
pub fn commit_changes(
    changes: impl IntoIterator<Item = Change>,
) -> Result<(), Vec<std::io::Error>> {
    let errors = changes
        .into_iter()
        .filter_map(|change| std::fs::write(change.file_name, change.proposed_content).err())
        .collect::<Vec<_>>();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn identifier_to_definition() {
        let src = b"fn foo(); fn foo -> huk { barf; } constant FOO: i32 = 42;";
        //          012345678901234567890123456789012345678901234567890123456
        //                    1         2         3         4         5
        let pos =
            rust_identifiers_to_definitions(src, [0usize, 4usize, 12, 19, 40]).collect::<Vec<_>>();
        assert_eq!(pos, vec![0..10, 0..10, 10..34, 10..34, 34..57]);
    }

    #[test]
    fn deletion() {
        let src = b"fn foo(); fn foo -> huk { barf; } constant FOO: i32 = 42;";
        //          012345678901234567890123456789012345678901234567890123456
        //                    1         2         3         4         5
        assert_eq!(
            rust_delete(src, [2usize]),
            b"fn foo -> huk { barf; } constant FOO: i32 = 42;"
        );
        assert_eq!(
            rust_delete(src, [10usize]),
            b"fn foo(); constant FOO: i32 = 42;"
        );
        assert_eq!(
            rust_delete(src, [40usize]),
            b"fn foo(); fn foo -> huk { barf; } "
        );
    }

    #[test]
    fn formatting_preserval() {
        let src = b" fn foo();  fn foo  -> huk {  barf; }   constant FOO: i32 = 42;  fn bar(){ } ";
        //          01234567890123456789012345678901234567890123456789012345678901234567890123456
        //                    1         2         3         4         5         6
        // 7
        assert_eq!(
            rust_delete(src, [5usize]),
            b"fn foo  -> huk {  barf; }   constant FOO: i32 = 42;  fn bar(){ } "
        );
        assert_eq!(
            rust_delete(src, [15usize]),
            b" fn foo();  constant FOO: i32 = 42;  fn bar(){ } "
        );
        assert_eq!(
            rust_delete(src, [42usize]),
            b" fn foo();  fn foo  -> huk {  barf; }   fn bar(){ } "
        );
        assert_eq!(
            rust_delete(src, [70usize]),
            b" fn foo();  fn foo  -> huk {  barf; }   constant FOO: i32 = 42;  "
        );

        assert_eq!(
            rust_delete(src, [5usize, 15]),
            b"constant FOO: i32 = 42;  fn bar(){ } "
        );
        assert_eq!(
            rust_delete(src, [15usize, 42usize]),
            b" fn foo();  fn bar(){ } "
        );
    }

    #[test]
    #[rustfmt::skip]
    fn whitespace_semi_preserval() {
        let src = b" fn foo() {} fn fixme() {} fn main() {}";
        assert_eq!(
            rust_delete(src, [15usize]),
            b" fn foo() {} fn main() {}"
        );
        let src = b" fn foo() {} fn fixme() {}fn main() {}";
        assert_eq!(
            rust_delete(src, [15usize]),
            b" fn foo() {} fn main() {}"
        );
        let src = b" fn foo() {}fn fixme() {} fn main() {}";
        assert_eq!(
            rust_delete(src, [15usize]),
            b" fn foo() {}fn main() {}"
        );
        let src = b" fn foo() {}\nfn fixme() {}\nfn main() {}";
        assert_eq!(
            rust_delete(src, [15usize]),
            b" fn foo() {}\nfn main() {}"
        );
        let src = b" fn foo() {}\n\nfn fixme() {}\nfn main() {}";
        assert_eq!(
            rust_delete(src, [15usize]),
            b" fn foo() {}\n\nfn main() {}"
        );
        let src = b" fn foo() {}\nfn fixme() {}\n\nfn main() {}";
        assert_eq!(
            rust_delete(src, [15usize]),
            b" fn foo() {}\n\nfn main() {}"
        );
        let src = b" fn foo() {}\n\nfn fixme() {}\n\nfn main() {}";
        assert_eq!(
            rust_delete(src, [15usize]),
            b" fn foo() {}\n\n\nfn main() {}"
        );
        let src = b"fn foo() {}\n         fn fixme() {}\n   fn main() {}";
        assert_eq!(
            rust_delete(src, [17usize]),
            b"fn foo() {}\n   fn main() {}"
        );
    }
}
