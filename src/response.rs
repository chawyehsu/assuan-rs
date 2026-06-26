//! Assuan response writing.
//!
//! Provides free functions to write protocol response lines to an
//! [`io::Write`] sink. All functions enforce the [`MAX_LINE_SIZE`] limit and
//! return the number of bytes written on success.
//!
//! # Line formats
//!
//! | Function        | Format                          |
//! |-----------------|---------------------------------|
//! | [`write_data`]  | `D <percent-encoded-data>\n`    |
//! | [`write_ok`]    | `OK [message]\n`                |
//! | [`write_err`]   | `ERR code [message]\n`          |
//! | [`write_status`]| `S keyword [value]\n`           |

use std::io::Write;

use crate::MAX_LINE_SIZE;
use crate::error::{Error, ErrorCode};
use crate::percent;

/// Write a data line (`D <encoded-data>\n`).
///
/// The raw `data` is percent-encoded before writing. Returns the number of
/// bytes written (including the `D ` prefix and trailing `\n`).
///
/// Returns [`Error::LineTooLong`] if the encoded output would exceed
/// [`MAX_LINE_SIZE`].
pub fn write_data<W: Write>(w: &mut W, data: &[u8]) -> Result<usize, Error> {
    // Max encoded data: MAX_LINE_SIZE - "D " (2) - "\n" (1) = 997 bytes.
    let max_encoded = MAX_LINE_SIZE - 3;
    let encoded_len = percent::encoded_len(data.len());
    if encoded_len > max_encoded {
        return Err(Error::LineTooLong);
    }

    let mut buf = [0u8; MAX_LINE_SIZE];
    buf[0] = b'D';
    buf[1] = b' ';
    let n = percent::encode(data, &mut buf[2..]);
    buf[2 + n] = b'\n';
    let total = 2 + n + 1;

    w.write_all(&buf[..total]).map_err(Error::Io)?;
    Ok(total)
}

/// Write an OK line (`OK [message]\n`).
///
/// If `msg` is `Some`, the message is appended after `OK `. Returns the number
/// of bytes written.
///
/// Returns [`Error::LineTooLong`] if the line would exceed [`MAX_LINE_SIZE`].
pub fn write_ok<W: Write>(w: &mut W, msg: Option<&str>) -> Result<usize, Error> {
    match msg {
        None => {
            w.write_all(b"OK\n").map_err(Error::Io)?;
            Ok(3)
        }
        Some(msg) => {
            // "OK " (3) + msg + "\n" (1)
            let total = 3 + msg.len() + 1;
            if total > MAX_LINE_SIZE {
                return Err(Error::LineTooLong);
            }
            let mut buf = [0u8; MAX_LINE_SIZE];
            buf[0] = b'O';
            buf[1] = b'K';
            buf[2] = b' ';
            buf[3..3 + msg.len()].copy_from_slice(msg.as_bytes());
            buf[3 + msg.len()] = b'\n';
            w.write_all(&buf[..total]).map_err(Error::Io)?;
            Ok(total)
        }
    }
}

/// Write an ERR line (`ERR code [message]\n`).
///
/// Returns the number of bytes written.
///
/// Returns [`Error::LineTooLong`] if the line would exceed [`MAX_LINE_SIZE`].
pub fn write_err<W: Write>(w: &mut W, code: ErrorCode, msg: Option<&str>) -> Result<usize, Error> {
    // Format: "ERR " + code_digits + " " + msg + "\n"
    // Code is a u32, max 10 digits.
    let code_str = code.0.to_string();

    match msg {
        None => {
            // "ERR " (4) + code + "\n" (1)
            let total = 4 + code_str.len() + 1;
            if total > MAX_LINE_SIZE {
                return Err(Error::LineTooLong);
            }
            let mut buf = [0u8; MAX_LINE_SIZE];
            buf[0] = b'E';
            buf[1] = b'R';
            buf[2] = b'R';
            buf[3] = b' ';
            buf[4..4 + code_str.len()].copy_from_slice(code_str.as_bytes());
            buf[4 + code_str.len()] = b'\n';
            w.write_all(&buf[..total]).map_err(Error::Io)?;
            Ok(total)
        }
        Some(msg) => {
            // "ERR " (4) + code + " " (1) + msg + "\n" (1)
            let total = 4 + code_str.len() + 1 + msg.len() + 1;
            if total > MAX_LINE_SIZE {
                return Err(Error::LineTooLong);
            }
            let mut buf = [0u8; MAX_LINE_SIZE];
            let mut pos = 0;
            buf[pos] = b'E';
            pos += 1;
            buf[pos] = b'R';
            pos += 1;
            buf[pos] = b'R';
            pos += 1;
            buf[pos] = b' ';
            pos += 1;
            buf[pos..pos + code_str.len()].copy_from_slice(code_str.as_bytes());
            pos += code_str.len();
            buf[pos] = b' ';
            pos += 1;
            buf[pos..pos + msg.len()].copy_from_slice(msg.as_bytes());
            pos += msg.len();
            buf[pos] = b'\n';
            pos += 1;
            w.write_all(&buf[..pos]).map_err(Error::Io)?;
            Ok(pos)
        }
    }
}

/// Write a status line (`S keyword [value]\n`).
///
/// Returns the number of bytes written.
///
/// Returns [`Error::LineTooLong`] if the line would exceed [`MAX_LINE_SIZE`].
pub fn write_status<W: Write>(w: &mut W, keyword: &str, value: &str) -> Result<usize, Error> {
    if value.is_empty() {
        // "S " (2) + keyword + "\n" (1)
        let total = 2 + keyword.len() + 1;
        if total > MAX_LINE_SIZE {
            return Err(Error::LineTooLong);
        }
        let mut buf = [0u8; MAX_LINE_SIZE];
        buf[0] = b'S';
        buf[1] = b' ';
        buf[2..2 + keyword.len()].copy_from_slice(keyword.as_bytes());
        buf[2 + keyword.len()] = b'\n';
        w.write_all(&buf[..total]).map_err(Error::Io)?;
        Ok(total)
    } else {
        // "S " (2) + keyword + " " (1) + value + "\n" (1)
        let total = 2 + keyword.len() + 1 + value.len() + 1;
        if total > MAX_LINE_SIZE {
            return Err(Error::LineTooLong);
        }
        let mut buf = [0u8; MAX_LINE_SIZE];
        let mut pos = 0;
        buf[pos] = b'S';
        pos += 1;
        buf[pos] = b' ';
        pos += 1;
        buf[pos..pos + keyword.len()].copy_from_slice(keyword.as_bytes());
        pos += keyword.len();
        buf[pos] = b' ';
        pos += 1;
        buf[pos..pos + value.len()].copy_from_slice(value.as_bytes());
        pos += value.len();
        buf[pos] = b'\n';
        pos += 1;
        w.write_all(&buf[..pos]).map_err(Error::Io)?;
        Ok(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(f: impl FnOnce(&mut Vec<u8>) -> Result<usize, Error>) -> (Vec<u8>, usize) {
        let mut buf = Vec::new();
        let n = f(&mut buf).unwrap();
        (buf, n)
    }

    // -- write_data --

    #[test]
    fn data_simple() {
        let (out, n) = collect(|w| write_data(w, b"hello"));
        assert_eq!(out, b"D hello\n");
        assert_eq!(n, 8);
    }

    #[test]
    fn data_with_encoding() {
        let (out, _) = collect(|w| write_data(w, b"hello\nworld"));
        assert_eq!(out, b"D hello%0Aworld\n");
    }

    #[test]
    fn data_with_percent() {
        let (out, _) = collect(|w| write_data(w, b"100%"));
        assert_eq!(out, b"D 100%25\n");
    }

    #[test]
    fn data_empty() {
        let (out, _) = collect(|w| write_data(w, b""));
        assert_eq!(out, b"D \n");
    }

    #[test]
    fn data_too_long() {
        // Each byte could encode to 3 bytes. 334 * 3 = 1002 > 997.
        let data = vec![b'\n'; 334];
        let mut out = Vec::new();
        assert!(matches!(
            write_data(&mut out, &data),
            Err(Error::LineTooLong)
        ));
    }

    // -- write_ok --

    #[test]
    fn ok_no_message() {
        let (out, n) = collect(|w| write_ok(w, None));
        assert_eq!(out, b"OK\n");
        assert_eq!(n, 3);
    }

    #[test]
    fn ok_with_message() {
        let (out, n) = collect(|w| write_ok(w, Some("my protocol version is 1")));
        assert_eq!(out, b"OK my protocol version is 1\n");
        assert_eq!(n, 28);
    }

    // -- write_err --

    #[test]
    fn err_no_message() {
        let (out, n) = collect(|w| write_err(w, ErrorCode::GENERAL, None));
        assert_eq!(out, b"ERR 1\n");
        assert_eq!(n, 6);
    }

    #[test]
    fn err_with_message() {
        let (out, n) =
            collect(|w| write_err(w, ErrorCode::ASS_UNKNOWN_CMD, Some("unknown command")));
        assert_eq!(out, b"ERR 275 unknown command\n");
        assert_eq!(n, 24);
    }

    // -- write_status --

    #[test]
    fn status_no_value() {
        let (out, n) = collect(|w| write_status(w, "PASSWORD_FROM_CACHE", ""));
        assert_eq!(out, b"S PASSWORD_FROM_CACHE\n");
        assert_eq!(n, 22);
    }

    #[test]
    fn status_with_value() {
        let (out, n) = collect(|w| write_status(w, "BUTTON_INFO", "close"));
        assert_eq!(out, b"S BUTTON_INFO close\n");
        assert_eq!(n, 20);
    }

    // -- line too long --

    #[test]
    fn ok_too_long() {
        let msg = "x".repeat(998);
        let mut out = Vec::new();
        assert!(matches!(
            write_ok(&mut out, Some(&msg)),
            Err(Error::LineTooLong)
        ));
    }
}
