//! Assuan percent-encoding and decoding.
//!
//! The Assuan protocol percent-encodes bytes outside `0x20..=0x7E` and the `%`
//! character itself. There is no `+`-for-space convention (unlike URL encoding).

/// Encode `src` into `dst`, returning the number of bytes written to `dst`.
///
/// Bytes in the printable ASCII range `0x20..=0x7E` (except `%`) are copied
/// literally. `%` and bytes outside that range are encoded as `%XX`.
///
/// # Panics
///
/// Panics if `dst` is too small to hold the encoded result.
pub fn encode(src: &[u8], dst: &mut [u8]) -> usize {
    let mut pos = 0;
    for &b in src {
        match b {
            0x20..=0x7E if b != b'%' => {
                dst[pos] = b;
                pos += 1;
            }
            _ => {
                dst[pos] = b'%';
                dst[pos + 1] = hex_upper(b >> 4);
                dst[pos + 2] = hex_upper(b & 0x0F);
                pos += 3;
            }
        }
    }
    pos
}

/// Return the maximum encoded length for `src_len` bytes.
///
/// Worst case: every byte is encoded as `%XX` (3 bytes each).
pub const fn encoded_len(src_len: usize) -> usize {
    src_len * 3
}

/// Decode percent-encoded bytes in-place, returning the decoded slice as `&str`.
///
/// Handles `%XX` escape sequences. Does NOT interpret `+` as space.
///
/// Returns an error if the decoded
/// bytes are not valid UTF-8.
pub fn decode_in_place(s: &mut [u8]) -> Result<&str, crate::Error> {
    let len = do_decode(s);
    std::str::from_utf8(&s[..len]).map_err(|_| crate::Error::new(crate::ErrorCode::ASS_INV_VALUE, "malformed line"))
}

/// In-place percent decoding. Returns the new length.
fn do_decode(bytes: &mut [u8]) -> usize {
    let mut write = 0;
    let mut read = 0;
    while read < bytes.len() {
        if bytes[read] == b'%' && read + 2 < bytes.len()
            && let (Some(hi), Some(lo)) = (hex_val(bytes[read + 1]), hex_val(bytes[read + 2]))
        {
            bytes[write] = (hi << 4) | lo;
            read += 3;
            write += 1;
            continue;
        }
        bytes[write] = bytes[read];
        read += 1;
        write += 1;
    }
    write
}

/// Convert a nibble (0–15) to its uppercase hex ASCII digit.
const fn hex_upper(b: u8) -> u8 {
    match b {
        0..=9 => b'0' + b,
        10..=15 => b'A' + b - 10,
        _ => unreachable!(),
    }
}

/// Convert an ASCII hex digit to its nibble value.
fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- encode tests --

    #[test]
    fn encode_ascii() {
        let src = b"hello";
        let mut dst = [0u8; 15];
        let n = encode(src, &mut dst);
        assert_eq!(&dst[..n], b"hello");
    }

    #[test]
    fn encode_space_literal() {
        // Spaces are NOT encoded in Assuan protocol.
        let src = b"hello world";
        let mut dst = [0u8; 33];
        let n = encode(src, &mut dst);
        assert_eq!(&dst[..n], b"hello world");
    }

    #[test]
    fn encode_percent() {
        let src = b"100%";
        let mut dst = [0u8; 12];
        let n = encode(src, &mut dst);
        assert_eq!(&dst[..n], b"100%25");
    }

    #[test]
    fn encode_newline() {
        let src = b"line1\nline2";
        let mut dst = [0u8; 33];
        let n = encode(src, &mut dst);
        assert_eq!(&dst[..n], b"line1%0Aline2");
    }

    #[test]
    fn encode_non_ascii() {
        let src = &[0x80u8, 0xFF];
        let mut dst = [0u8; 6];
        let n = encode(src, &mut dst);
        assert_eq!(&dst[..n], b"%80%FF");
    }

    #[test]
    fn encode_empty() {
        let src = b"";
        let mut dst = [0u8; 0];
        let n = encode(src, &mut dst);
        assert_eq!(n, 0);
    }

    // -- decode tests --

    #[test]
    fn decode_no_encoding() {
        let mut s = b"hello".to_vec();
        assert_eq!(decode_in_place(&mut s).unwrap(), "hello");
    }

    #[test]
    fn decode_percent_encoding() {
        let mut s = b"hello%20world".to_vec();
        assert_eq!(decode_in_place(&mut s).unwrap(), "hello world");
    }

    #[test]
    fn decode_newline() {
        let mut s = b"line1%0Aline2".to_vec();
        assert_eq!(decode_in_place(&mut s).unwrap(), "line1\nline2");
    }

    #[test]
    fn decode_percent_literal() {
        let mut s = b"100%25".to_vec();
        assert_eq!(decode_in_place(&mut s).unwrap(), "100%");
    }

    #[test]
    fn decode_empty() {
        let mut s = b"".to_vec();
        assert_eq!(decode_in_place(&mut s).unwrap(), "");
    }

    #[test]
    fn decode_lowercase_hex() {
        let mut s = b"line1%0aline2".to_vec();
        assert_eq!(decode_in_place(&mut s).unwrap(), "line1\nline2");
    }

    #[test]
    fn decode_invalid_percent_at_end() {
        // Incomplete %XX at end — left as-is per spec behavior.
        let mut s = b"test%".to_vec();
        assert_eq!(decode_in_place(&mut s).unwrap(), "test%");
    }

    #[test]
    fn decode_invalid_hex_after_percent() {
        // %ZZ is not valid hex — left as-is.
        let mut s = b"test%ZZ".to_vec();
        assert_eq!(decode_in_place(&mut s).unwrap(), "test%ZZ");
    }

    // -- round-trip test --

    #[test]
    fn encode_decode_roundtrip() {
        // Only test inputs whose decoded form is valid UTF-8.
        let inputs: &[&[u8]] = &[
            b"hello world",
            b"100%",
            b"line1\nline2",
            b"\x00\x01\x02",
            b"",
            b"Enter passphrase to unlock",
        ];
        for input in inputs {
            let mut encoded = vec![0u8; encoded_len(input.len())];
            let n = encode(input, &mut encoded);
            encoded.truncate(n);
            let decoded = decode_in_place(&mut encoded).unwrap();
            assert_eq!(decoded.as_bytes(), *input, "roundtrip failed for {:?}", input);
        }
    }

    #[test]
    fn decode_non_utf8_returns_error() {
        // 0xFF is not valid UTF-8, so decoding %FF must fail.
        let mut encoded = b"%FF".to_vec();
        assert!(decode_in_place(&mut encoded).is_err());
    }
}
