//! Internal line writer for the Assuan protocol.
//!
//! Writes complete lines to an [`io::Write`] sink, enforcing the
//! [`MAX_LINE_SIZE`] limit and appending a trailing `\n`.

use std::io::Write;

use crate::MAX_LINE_SIZE;
use crate::error::{Error, ErrorCode};
use crate::percent;

/// Buffered line writer with a fixed internal buffer.
///
/// Writes complete Assuan protocol lines to an underlying [`io::Write`].
/// Enforces the 1000-byte line length limit.
pub(crate) struct LineWriter<W: Write> {
    writer: W,
}

impl<W: Write> LineWriter<W> {
    /// Create a new line writer wrapping the given writer.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write a raw line, appending `\n`.
    ///
    /// The line must not exceed [`MAX_LINE_SIZE`] bytes (including the `\n`).
    /// Returns the number of bytes written.
    pub fn write_line(&mut self, line: &[u8]) -> Result<usize, Error> {
        let total = line.len() + 1; // +1 for \n
        if total > MAX_LINE_SIZE {
            return Err(Error::LineTooLong);
        }
        self.writer.write_all(line).map_err(Error::Io)?;
        self.writer.write_all(b"\n").map_err(Error::Io)?;
        Ok(total)
    }

    /// Write a data line (`D <percent-encoded-data>\n`).
    ///
    /// The raw `data` is percent-encoded before writing. Returns the number
    /// of bytes written.
    pub fn write_data(&mut self, data: &[u8]) -> Result<usize, Error> {
        let max_encoded = MAX_LINE_SIZE - 3; // "D " (2) + "\n" (1)
        let encoded_len = percent::encoded_len(data.len());
        if encoded_len > max_encoded {
            return Err(Error::LineTooLong);
        }

        let mut buf = [0u8; MAX_LINE_SIZE - 1]; // leave room for \n in write_line
        buf[0] = b'D';
        buf[1] = b' ';
        let n = percent::encode(data, &mut buf[2..]);
        self.write_line(&buf[..2 + n])
    }

    /// Write an OK line (`OK [message]\n`).
    pub fn write_ok(&mut self, msg: Option<&str>) -> Result<usize, Error> {
        match msg {
            None => self.write_line(b"OK"),
            Some(msg) => {
                let line = format!("OK {msg}");
                self.write_line(line.as_bytes())
            }
        }
    }

    /// Write an ERR line (`ERR code [message]\n`).
    pub fn write_err(&mut self, code: ErrorCode, msg: Option<&str>) -> Result<usize, Error> {
        match msg {
            None => {
                let line = format!("ERR {code}");
                self.write_line(line.as_bytes())
            }
            Some(msg) => {
                let line = format!("ERR {code} {msg}");
                self.write_line(line.as_bytes())
            }
        }
    }

    /// Write a status line (`S keyword [value]\n`).
    pub fn write_status(&mut self, keyword: &str, value: &str) -> Result<usize, Error> {
        if value.is_empty() {
            let line = format!("S {keyword}");
            self.write_line(line.as_bytes())
        } else {
            let line = format!("S {keyword} {value}");
            self.write_line(line.as_bytes())
        }
    }

    /// Write a comment line (`# comment\n`).
    pub fn write_comment(&mut self, comment: &str) -> Result<usize, Error> {
        let line = format!("# {comment}");
        self.write_line(line.as_bytes())
    }

    /// Write an INQUIRE line (`INQUIRE keyword [params]\n`).
    pub fn write_inquire(&mut self, keyword: &str, params: &str) -> Result<usize, Error> {
        if params.is_empty() {
            let line = format!("INQUIRE {keyword}");
            self.write_line(line.as_bytes())
        } else {
            let line = format!("INQUIRE {keyword} {params}");
            self.write_line(line.as_bytes())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(f: impl FnOnce(&mut LineWriter<Vec<u8>>) -> Result<usize, Error>) -> (Vec<u8>, usize) {
        let mut lw = LineWriter::new(Vec::new());
        let n = f(&mut lw).unwrap();
        (lw.writer, n)
    }

    // -- write_line --

    #[test]
    fn write_line_simple() {
        let (out, n) = collect(|lw| lw.write_line(b"OK"));
        assert_eq!(out, b"OK\n");
        assert_eq!(n, 3);
    }

    #[test]
    fn write_line_empty() {
        let (out, n) = collect(|lw| lw.write_line(b""));
        assert_eq!(out, b"\n");
        assert_eq!(n, 1);
    }

    #[test]
    fn write_line_max_length() {
        // 999 bytes + 1 newline = 1000 (exactly MAX_LINE_SIZE)
        let line = vec![b'x'; 999];
        let (out, n) = collect(|lw| lw.write_line(&line));
        assert_eq!(n, 1000);
        assert_eq!(out.len(), 1000);
        assert_eq!(out[999], b'\n');
    }

    #[test]
    fn write_line_too_long() {
        // 1000 bytes + 1 newline = 1001 > MAX_LINE_SIZE
        let line = vec![b'x'; 1000];
        let mut lw = LineWriter::new(Vec::new());
        assert!(matches!(lw.write_line(&line), Err(Error::LineTooLong)));
    }

    #[test]
    fn write_line_multiple() {
        let mut lw = LineWriter::new(Vec::new());
        lw.write_line(b"line1").unwrap();
        lw.write_line(b"line2").unwrap();
        assert_eq!(lw.writer, b"line1\nline2\n");
    }

    // -- write_data --

    #[test]
    fn write_data_simple() {
        let (out, n) = collect(|lw| lw.write_data(b"hello"));
        assert_eq!(out, b"D hello\n");
        assert_eq!(n, 8);
    }

    #[test]
    fn write_data_empty() {
        let (out, n) = collect(|lw| lw.write_data(b""));
        assert_eq!(out, b"D \n");
        assert_eq!(n, 3);
    }

    #[test]
    fn write_data_with_encoding() {
        let (out, _) = collect(|lw| lw.write_data(b"hello\nworld"));
        assert_eq!(out, b"D hello%0Aworld\n");
    }

    #[test]
    fn write_data_with_percent() {
        let (out, _) = collect(|lw| lw.write_data(b"100%"));
        assert_eq!(out, b"D 100%25\n");
    }

    #[test]
    fn write_data_too_long() {
        // Each \n encodes to 3 bytes (%0A). 334 * 3 = 1002 > 997.
        let data = vec![b'\n'; 334];
        let mut lw = LineWriter::new(Vec::new());
        assert!(matches!(lw.write_data(&data), Err(Error::LineTooLong)));
    }

    #[test]
    fn write_data_non_ascii() {
        let (out, _) = collect(|lw| lw.write_data(&[0x80, 0xFF]));
        assert_eq!(out, b"D %80%FF\n");
    }

    // -- write_ok --

    #[test]
    fn write_ok_no_message() {
        let (out, n) = collect(|lw| lw.write_ok(None));
        assert_eq!(out, b"OK\n");
        assert_eq!(n, 3);
    }

    #[test]
    fn write_ok_with_message() {
        let (out, n) = collect(|lw| lw.write_ok(Some("protocol version 1")));
        assert_eq!(out, b"OK protocol version 1\n");
        assert_eq!(n, 22);
    }

    // -- write_err --

    #[test]
    fn write_err_no_message() {
        let (out, n) = collect(|lw| lw.write_err(ErrorCode::GENERAL, None));
        assert_eq!(out, b"ERR 1\n");
        assert_eq!(n, 6);
    }

    #[test]
    fn write_err_with_message() {
        let (out, n) = collect(|lw| lw.write_err(ErrorCode::ASS_UNKNOWN_CMD, Some("unknown")));
        assert_eq!(out, b"ERR 275 unknown\n");
        assert_eq!(n, 16);
    }

    // -- write_status --

    #[test]
    fn write_status_no_value() {
        let (out, n) = collect(|lw| lw.write_status("PASSWORD_FROM_CACHE", ""));
        assert_eq!(out, b"S PASSWORD_FROM_CACHE\n");
        assert_eq!(n, 22);
    }

    #[test]
    fn write_status_with_value() {
        let (out, n) = collect(|lw| lw.write_status("BUTTON_INFO", "close"));
        assert_eq!(out, b"S BUTTON_INFO close\n");
        assert_eq!(n, 20);
    }

    // -- write_comment --

    #[test]
    fn write_comment_simple() {
        let (out, n) = collect(|lw| lw.write_comment("debug info"));
        assert_eq!(out, b"# debug info\n");
        assert_eq!(n, 13);
    }

    // -- write_inquire --

    #[test]
    fn write_inquire_no_params() {
        let (out, n) = collect(|lw| lw.write_inquire("PASSPHRASE", ""));
        assert_eq!(out, b"INQUIRE PASSPHRASE\n");
        assert_eq!(n, 19);
    }

    #[test]
    fn write_inquire_with_params() {
        let (out, n) = collect(|lw| lw.write_inquire("PASSPHRASE", "key=abc"));
        assert_eq!(out, b"INQUIRE PASSPHRASE key=abc\n");
        assert_eq!(n, 27);
    }
}