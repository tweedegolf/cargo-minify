use std::ops::Range;

/// Turns a list of "locations of identifiers" into a list of "chunk
fn rust_identifiers_to_definitions<'a>(
    src: &'a [u8],
    locations: impl IntoIterator<Item = usize> + 'a,
) -> impl Iterator<Item = Range<usize>> + 'a {
    locations.into_iter().map(|pos| {
        let prev = src[..pos]
            .iter()
            .rposition(|x| b";}".contains(x))
            .map(|i| i + 1)
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
                    // TODO: graceful erroring for unmatched pairs
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

                i
            })
            .unwrap_or(src.len());

        prev..next
    })
}

pub fn rust_delete(src: &[u8], locations: impl IntoIterator<Item = usize>) -> Vec<u8> {
    todo!()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_identifier_to_definition() {
        let src = b"fn foo(); fn foo -> huk { barf; } constant FOO: i32 = 42;";
        //                  012345678901234567890123456789012345678901234567890123456
        //                            1         2         3         4         5
        let pos =
            rust_identifiers_to_definitions(src, [0usize, 4usize, 12, 19, 40]).collect::<Vec<_>>();
        assert_eq!(pos, vec![0..9, 0..9, 9..33, 9..33, 33..57]);
    }

    #[test]
    fn deletion_test() {
        let src = b"fn foo(); fn foo -> huk { barf; } constant FOO: i32 = 42;";
        //                  012345678901234567890123456789012345678901234567890123456
        //                            1         2         3         4         5
        let out = rust_delete(src, [2usize]);
        assert_eq!(out, b"fn foo -> huk { barf; } constant FOO: i32 = 42;");
    }
}
