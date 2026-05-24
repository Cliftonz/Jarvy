//! RFC 3986 unreserved-set percent-encoder.
//!
//! Encodes every byte outside `[A-Za-z0-9_.~-]` as `%XX`. UTF-8 input is
//! handled correctly by iterating bytes — the unreserved set is ASCII,
//! so multi-byte characters are always encoded one byte at a time, which
//! is the spec behavior.
//!
//! Kept inline (no `percent_encoding` crate dependency) per the project
//! preference for stdlib over new deps — see `CLAUDE.md`.

use std::fmt::Write as _;

/// Percent-encode `input` using the RFC 3986 unreserved-character set,
/// writing directly into `out`. The in-place form avoids the
/// throwaway `String` allocation that `encode_unreserved` returns —
/// preferred when building a URL by appending multiple encoded
/// segments to an existing buffer (see `unsupported::issue_url`).
pub fn encode_unreserved_into(out: &mut String, input: &str) {
    // Pre-extend by the input length; any escape grows by 2 extra
    // bytes (`%XX`) which the inner `write!` handles. Doing one
    // reserve up-front avoids multiple reallocs on long inputs.
    out.reserve(input.len());
    for b in input.bytes() {
        let safe = b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~');
        if safe {
            out.push(b as char);
        } else {
            // write! goes straight into `out` — avoids the `format!`
            // intermediate `String` allocation per escaped byte.
            let _ = write!(out, "%{:02X}", b);
        }
    }
}

/// Percent-encode `input` using the RFC 3986 unreserved-character set.
///
/// Suitable for URL query-string values and path segments where the
/// safe-character set is `[A-Za-z0-9_.~-]`. Returns a freshly allocated
/// `String`. Pre-sizes the output to fit the worst-case escape ratio
/// for the unsafe-byte count (one pass to count, second pass to
/// encode) so the result avoids reallocs.
pub fn encode_unreserved(input: &str) -> String {
    let unsafe_count = input
        .bytes()
        .filter(|b| !b.is_ascii_alphanumeric() && !matches!(*b, b'-' | b'_' | b'.' | b'~'))
        .count();
    let mut out = String::with_capacity(input.len() + 2 * unsafe_count);
    encode_unreserved_into(&mut out, input);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_unreserved() {
        assert_eq!(encode_unreserved("abc-XYZ_0.9~"), "abc-XYZ_0.9~");
    }

    #[test]
    fn escapes_space_and_brackets() {
        assert_eq!(
            encode_unreserved("[Tool]: foo bar"),
            "%5BTool%5D%3A%20foo%20bar"
        );
    }

    #[test]
    fn escapes_multibyte_utf8_byte_by_byte() {
        // U+65E5 ('日') is 3 bytes: 0xE6 0x97 0xA5 in UTF-8.
        assert_eq!(encode_unreserved("日"), "%E6%97%A5");
    }

    #[test]
    fn escapes_crlf() {
        assert_eq!(encode_unreserved("\r\n"), "%0D%0A");
    }
}
