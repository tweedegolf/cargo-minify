use std::{ops::Range, path::Path};

use cargo_metadata::diagnostic::DiagnosticSpan;

/// Turns a list of "locations of identifiers" into a list of "chunk
/// BUGS: this is not safe for use on macros
fn rust_identifiers_to_definitions<'a>(
    src: &'a [u8],
    locations: impl IntoIterator<Item = usize> + 'a,
) -> impl Iterator<Item = Range<usize>> + 'a {
    locations.into_iter().map(|pos| {
        let prev = src[..pos]
            .iter()
            .rposition(|x| b";}".contains(x))
            .map(|i| {
                if src[i + 1].is_ascii_whitespace() {
                    i + 2
                } else {
                    i + 1
                }
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
                    if i == src.len() {
                        return i;
                    }

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

                    if level == 0 {
                        break;
                    }
                }

                src[i..]
                    .iter()
                    .position(|c| !c.is_ascii_whitespace())
                    .map(|k| i + k)
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
fn process_files<'a, Iter: IntoIterator<Item = usize>>(
    diagnostics: impl Iterator<Item = (&'a str, Iter)>,
) -> impl Iterator<Item = (&'a str, Vec<u8>)> {
    diagnostics.filter_map(|(source, byte_locations)| {
        let bytes = std::fs::read(source).ok()?;

        Some((source, rust_delete(&bytes, byte_locations)))
    })
}

/// Process a list of UnusedDiagnostics into an iterator of filenames+proposed
/// contents BUGS: this does not check that the diagnostic is a "unused
/// diagnostic"
pub fn process_diagnostics<'a>(
    diagnostics: impl IntoIterator<Item = &'a DiagnosticSpan>,
) -> impl Iterator<Item = (&'a str, Vec<u8>)> {
    process_files(
        diagnostics
            .into_iter()
            .map(|span| (span.file_name.as_str(), span.byte_start as usize))
            .collect::<multimap::MultiMap<_, _>>()
            .into_iter(),
    )
}

/// DANGER
pub fn commit_changes(
    changes: impl IntoIterator<Item = (impl AsRef<Path>, Vec<u8>)>,
) -> Result<(), Vec<std::io::Error>> {
    let errors = changes
        .into_iter()
        .filter_map(|(file, contents)| std::fs::write(file, contents).err())
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
    fn test_identifier_to_definition() {
        let src = b"fn foo(); fn foo -> huk { barf; } constant FOO: i32 = 42;";
        //          012345678901234567890123456789012345678901234567890123456
        //                    1         2         3         4         5
        let pos =
            rust_identifiers_to_definitions(src, [0usize, 4usize, 12, 19, 40]).collect::<Vec<_>>();
        assert_eq!(pos, vec![0..10, 0..10, 10..34, 10..34, 34..57]);
    }

    #[test]
    fn deletion_test() {
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
}
