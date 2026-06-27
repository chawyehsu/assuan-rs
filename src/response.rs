//! Assuan response types and writing.
//!
//! The [`Response`] enum represents all server-side response lines in the Assuan
//! protocol. Each variant carries owned data and can be written to any
//! [`io::Write`] sink via [`Response::write_to`].
//!
//! # Line formats
//!
//! | Variant         | Format                          |
//! |-----------------|---------------------------------|
//! | `Ok`            | `OK [message]\n`                |
//! | `Err`           | `ERR code [message]\n`          |
//! | `Status`        | `S keyword [value]\n`           |
//! | `Comment`       | `# comment\n`                   |
//! | `Data`          | `D <percent-encoded-data>\n`    |
//! | `Inquire`       | `INQUIRE keyword [params]\n`    |

use std::fmt;
use std::io::Write;

use crate::MAX_LINE_SIZE;
use crate::error::{Error, ErrorCode};
use crate::percent;

/// An Assuan server response line.
///
/// All data is owned. Use the convenience constructors or `const` values for
/// common responses:
///
/// ```
/// use assuan::Response;
///
/// // Zero-alloc OK with no message
/// let ok = Response::OK;
///
/// // OK with a message (allocates)
/// let ok = Response::ok("protocol version 1");
///
/// // Error with code and message
/// let err = Response::Err(assuan::ErrorCode::GENERAL, Some("oops".into()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    /// OK response with an optional message.
    Ok(Option<String>),

    /// ERR response with an error code and optional message.
    Err(ErrorCode, Option<String>),

    /// S (status) response with a keyword and value.
    Status(String, String),

    /// # comment line (debugging only, ignored by the peer).
    Comment(String),

    /// D data line — percent-encoded on the wire.
    Data(Vec<u8>),

    /// INQUIRE — server requests data from the client.
    Inquire(String, String),
}

impl Response {
    /// OK with no message — zero-alloc, const-constructible.
    pub const OK: Self = Response::Ok(None);

    /// Create an OK response with a message.
    pub fn ok(msg: impl Into<String>) -> Self {
        Response::Ok(Some(msg.into()))
    }

    /// Create an ERR response with a code and optional message.
    pub fn err(code: ErrorCode, msg: Option<String>) -> Self {
        Response::Err(code, msg)
    }

    /// Create a status response.
    pub fn status(keyword: impl Into<String>, value: impl Into<String>) -> Self {
        Response::Status(keyword.into(), value.into())
    }

    /// Create a data response.
    pub fn data(bytes: Vec<u8>) -> Self {
        Response::Data(bytes)
    }

    /// Create an INQUIRE response.
    pub fn inquire(keyword: impl Into<String>, params: impl Into<String>) -> Self {
        Response::Inquire(keyword.into(), params.into())
    }

    /// Parse a raw line into a response.
    ///
    /// The input should come from [`LineReader::read_line`](crate::LineReader::read_line),
    /// which already strips trailing LF/CRLF.
    pub fn parse(line: &mut [u8]) -> Result<Self, Error> {
        if line.is_empty() {
            return Err(Error::LineMalformed);
        }

        let line_str = std::str::from_utf8(line).map_err(|_| Error::LineMalformed)?;

        // OK [message]
        if let Some(rest) = line_str.strip_prefix("OK") {
            let msg = rest.strip_prefix(' ').map(|s| s.to_string());
            return Ok(Response::Ok(msg));
        }

        // ERR code [message]
        if let Some(rest) = line_str.strip_prefix("ERR ") {
            let mut parts = rest.splitn(2, ' ');
            let code_str = parts.next().unwrap_or("");
            let code: u32 = code_str.parse().map_err(|_| Error::LineMalformed)?;
            let msg = parts.next().map(|s| s.to_string());
            return Ok(Response::Err(ErrorCode(code), msg));
        }

        // S keyword [value]
        if let Some(rest) = line_str.strip_prefix("S ") {
            let mut parts = rest.splitn(2, ' ');
            let keyword = parts.next().unwrap_or("").to_string();
            let value = parts.next().unwrap_or("").to_string();
            return Ok(Response::Status(keyword, value));
        }

        // # comment
        if let Some(rest) = line_str.strip_prefix("# ") {
            return Ok(Response::Comment(rest.to_string()));
        }

        // D data (percent-encoded)
        if let Some(rest) = line_str.strip_prefix("D ") {
            let mut data_bytes = rest.as_bytes().to_vec();
            let decoded = percent::decode_in_place(&mut data_bytes)?;
            return Ok(Response::Data(decoded.as_bytes().to_vec()));
        }
        if line_str == "D" {
            return Ok(Response::Data(vec![]));
        }

        // INQUIRE keyword [params]
        if let Some(rest) = line_str.strip_prefix("INQUIRE ") {
            let mut parts = rest.splitn(2, ' ');
            let keyword = parts.next().unwrap_or("").to_string();
            let params = parts.next().unwrap_or("").to_string();
            return Ok(Response::Inquire(keyword, params));
        }

        Err(Error::LineMalformed)
    }

    /// Write this response to the given writer.
    ///
    /// Returns the number of bytes written. Enforces [`MAX_LINE_SIZE`].
    pub fn write_to<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        match self {
            Response::Ok(msg) => write_ok(w, msg.as_deref()),
            Response::Err(code, msg) => write_err(w, *code, msg.as_deref()),
            Response::Status(kw, val) => write_status(w, kw, val),
            Response::Comment(s) => write_comment(w, s),
            Response::Data(data) => write_data(w, data),
            Response::Inquire(kw, params) => write_inquire(w, kw, params),
        }
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Response::Ok(None) => write!(f, "OK"),
            Response::Ok(Some(msg)) => write!(f, "OK {msg}"),
            Response::Err(code, None) => write!(f, "ERR {code}"),
            Response::Err(code, Some(msg)) => write!(f, "ERR {code} {msg}"),
            Response::Status(kw, val) => {
                if val.is_empty() {
                    write!(f, "S {kw}")
                } else {
                    write!(f, "S {kw} {val}")
                }
            }
            Response::Comment(s) => write!(f, "# {s}"),
            Response::Data(_) => write!(f, "D <data>"),
            Response::Inquire(kw, params) => {
                if params.is_empty() {
                    write!(f, "INQUIRE {kw}")
                } else {
                    write!(f, "INQUIRE {kw} {params}")
                }
            }
        }
    }
}

// -- Internal write helpers -----------------------------------------------

/// Write an OK line (`OK [message]\n`).
fn write_ok<W: Write>(w: &mut W, msg: Option<&str>) -> Result<usize, Error> {
    match msg {
        None => {
            w.write_all(b"OK\n").map_err(Error::Io)?;
            Ok(3)
        }
        Some(msg) => {
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
fn write_err<W: Write>(w: &mut W, code: ErrorCode, msg: Option<&str>) -> Result<usize, Error> {
    let code_str = code.0.to_string();

    match msg {
        None => {
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
            let total = 4 + code_str.len() + 1 + msg.len() + 1;
            if total > MAX_LINE_SIZE {
                return Err(Error::LineTooLong);
            }
            let mut buf = [0u8; MAX_LINE_SIZE];
            let mut pos = 0;
            buf[pos] = b'E'; pos += 1;
            buf[pos] = b'R'; pos += 1;
            buf[pos] = b'R'; pos += 1;
            buf[pos] = b' '; pos += 1;
            buf[pos..pos + code_str.len()].copy_from_slice(code_str.as_bytes());
            pos += code_str.len();
            buf[pos] = b' '; pos += 1;
            buf[pos..pos + msg.len()].copy_from_slice(msg.as_bytes());
            pos += msg.len();
            buf[pos] = b'\n';
            w.write_all(&buf[..pos + 1]).map_err(Error::Io)?;
            Ok(pos + 1)
        }
    }
}

/// Write a status line (`S keyword [value]\n`).
fn write_status<W: Write>(w: &mut W, keyword: &str, value: &str) -> Result<usize, Error> {
    if value.is_empty() {
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
        let total = 2 + keyword.len() + 1 + value.len() + 1;
        if total > MAX_LINE_SIZE {
            return Err(Error::LineTooLong);
        }
        let mut buf = [0u8; MAX_LINE_SIZE];
        let mut pos = 0;
        buf[pos] = b'S'; pos += 1;
        buf[pos] = b' '; pos += 1;
        buf[pos..pos + keyword.len()].copy_from_slice(keyword.as_bytes());
        pos += keyword.len();
        buf[pos] = b' '; pos += 1;
        buf[pos..pos + value.len()].copy_from_slice(value.as_bytes());
        pos += value.len();
        buf[pos] = b'\n';
        w.write_all(&buf[..pos + 1]).map_err(Error::Io)?;
        Ok(pos + 1)
    }
}

/// Write a comment line (`# comment\n`).
fn write_comment<W: Write>(w: &mut W, comment: &str) -> Result<usize, Error> {
    let total = 2 + comment.len() + 1;
    if total > MAX_LINE_SIZE {
        return Err(Error::LineTooLong);
    }
    let mut buf = [0u8; MAX_LINE_SIZE];
    buf[0] = b'#';
    buf[1] = b' ';
    buf[2..2 + comment.len()].copy_from_slice(comment.as_bytes());
    buf[2 + comment.len()] = b'\n';
    w.write_all(&buf[..total]).map_err(Error::Io)?;
    Ok(total)
}

/// Write a data line (`D <percent-encoded-data>\n`).
fn write_data<W: Write>(w: &mut W, data: &[u8]) -> Result<usize, Error> {
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

/// Write an INQUIRE line (`INQUIRE keyword [params]\n`).
fn write_inquire<W: Write>(w: &mut W, keyword: &str, params: &str) -> Result<usize, Error> {
    if params.is_empty() {
        let line = format!("INQUIRE {keyword}\n");
        if line.len() > MAX_LINE_SIZE {
            return Err(Error::LineTooLong);
        }
        w.write_all(line.as_bytes()).map_err(Error::Io)?;
        Ok(line.len())
    } else {
        let line = format!("INQUIRE {keyword} {params}\n");
        if line.len() > MAX_LINE_SIZE {
            return Err(Error::LineTooLong);
        }
        w.write_all(line.as_bytes()).map_err(Error::Io)?;
        Ok(line.len())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    fn collect(resp: &Response) -> (Vec<u8>, usize) {
        let mut buf = Vec::new();
        let n = resp.write_to(&mut buf).unwrap();
        (buf, n)
    }

    // -- Response::Ok --

    #[test]
    fn ok_no_message() {
        let (out, n) = collect(&Response::OK);
        assert_eq!(out, b"OK\n");
        assert_eq!(n, 3);
    }

    #[test]
    fn ok_with_message() {
        let (out, n) = collect(&Response::ok("my protocol version is 1"));
        assert_eq!(out, b"OK my protocol version is 1\n");
        assert_eq!(n, 28);
    }

    #[test]
    fn ok_const() {
        // Response::OK is const-constructible
        const R: Response = Response::OK;
        let (out, _) = collect(&R);
        assert_eq!(out, b"OK\n");
    }

    // -- Response::Err --

    #[test]
    fn err_no_message() {
        let (out, n) = collect(&Response::err(ErrorCode::GENERAL, None));
        assert_eq!(out, b"ERR 1\n");
        assert_eq!(n, 6);
    }

    #[test]
    fn err_with_message() {
        let (out, n) = collect(&Response::err(
            ErrorCode::ASS_UNKNOWN_CMD,
            Some("unknown command".into()),
        ));
        assert_eq!(out, b"ERR 275 unknown command\n");
        assert_eq!(n, 24);
    }

    // -- Response::Status --

    #[test]
    fn status_no_value() {
        let (out, n) = collect(&Response::status("PASSWORD_FROM_CACHE", ""));
        assert_eq!(out, b"S PASSWORD_FROM_CACHE\n");
        assert_eq!(n, 22);
    }

    #[test]
    fn status_with_value() {
        let (out, n) = collect(&Response::status("BUTTON_INFO", "close"));
        assert_eq!(out, b"S BUTTON_INFO close\n");
        assert_eq!(n, 20);
    }

    // -- Response::Data --

    #[test]
    fn data_simple() {
        let (out, n) = collect(&Response::data(b"hello".to_vec()));
        assert_eq!(out, b"D hello\n");
        assert_eq!(n, 8);
    }

    #[test]
    fn data_with_encoding() {
        let (out, _) = collect(&Response::data(b"hello\nworld".to_vec()));
        assert_eq!(out, b"D hello%0Aworld\n");
    }

    #[test]
    fn data_with_percent() {
        let (out, _) = collect(&Response::data(b"100%".to_vec()));
        assert_eq!(out, b"D 100%25\n");
    }

    #[test]
    fn data_empty() {
        let (out, _) = collect(&Response::data(vec![]));
        assert_eq!(out, b"D \n");
    }

    #[test]
    fn data_too_long() {
        let data = vec![b'\n'; 334];
        let mut out = Vec::new();
        assert!(matches!(
            Response::data(data).write_to(&mut out),
            Err(Error::LineTooLong)
        ));
    }

    // -- Response::Comment --

    #[test]
    fn comment() {
        let (out, n) = collect(&Response::Comment("debug info".into()));
        assert_eq!(out, b"# debug info\n");
        assert_eq!(n, 13);
    }

    // -- Response::Inquire --

    #[test]
    fn inquire_no_params() {
        let (out, n) = collect(&Response::inquire("PASSPHRASE", ""));
        assert_eq!(out, b"INQUIRE PASSPHRASE\n");
        assert_eq!(n, 19);
    }

    #[test]
    fn inquire_with_params() {
        let (out, n) = collect(&Response::inquire("PASSPHRASE", "key=abc"));
        assert_eq!(out, b"INQUIRE PASSPHRASE key=abc\n");
        assert_eq!(n, 27);
    }

    // -- Display --

    #[test]
    fn display_ok() {
        assert_eq!(Response::OK.to_string(), "OK");
        assert_eq!(Response::ok("msg").to_string(), "OK msg");
    }

    #[test]
    fn display_err() {
        assert_eq!(
            Response::err(ErrorCode::GENERAL, None).to_string(),
            "ERR 1"
        );
        assert_eq!(
            Response::err(ErrorCode::GENERAL, Some("bad".into())).to_string(),
            "ERR 1 bad"
        );
    }

    // -- line too long --

    #[test]
    fn ok_too_long() {
        let msg = "x".repeat(998);
        let mut out = Vec::new();
        assert!(matches!(
            Response::ok(msg).write_to(&mut out),
            Err(Error::LineTooLong)
        ));
    }
}
