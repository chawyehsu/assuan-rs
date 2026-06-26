//! Assuan request parsing.
//!
//! Parses a raw line (from [`LineReader`](crate::LineReader)) into a structured
//! [`Request`] with command name and arguments. Zero allocations — modifies the
//! line buffer in-place.

use crate::Error;
use crate::percent;

/// A parsed Assuan request.
///
/// Borrows from the mutable line buffer returned by
/// [`LineReader::read_line`](crate::LineReader::read_line). The command name is
/// uppercased in-place; arguments are percent-decoded in-place.
///
/// # Example
///
/// ```
/// use assuan::Request;
///
/// let mut line = b"SETDESC Enter passphrase".to_vec();
/// let req = Request::parse(&mut line).unwrap().unwrap();
/// assert_eq!(req.command(), "SETDESC");
/// assert_eq!(req.args(), Some("Enter passphrase"));
/// ```
pub struct Request<'a> {
    command: &'a str,
    args: Option<&'a str>,
}

impl<'a> Request<'a> {
    /// Parse a raw line into a request.
    ///
    /// Returns `Ok(None)` for empty lines. Returns `Err(Error::LineMalformed)`
    /// if the line or its percent-decoded arguments are not valid UTF-8. The
    /// command name is uppercased in-place; arguments are percent-decoded
    /// in-place.
    ///
    /// Comment lines (`# ...`) are parsed as regular requests with command `"#"`.
    /// The consumer decides how to handle them.
    ///
    /// The input should come from [`LineReader::read_line`](crate::LineReader::read_line),
    /// which already strips trailing LF/CRLF.
    pub fn parse(line: &'a mut [u8]) -> Result<Option<Self>, Error> {
        if line.is_empty() {
            return Ok(None);
        }

        // Split into command and args at the first space.
        let space_pos = line.iter().position(|&b| b == b' ');
        let cmd_end = space_pos.unwrap_or(line.len());

        // Split into non-overlapping mutable slices: command and (optional) args.
        let (cmd_part, rest) = line.split_at_mut(cmd_end);

        // Uppercase the command name in-place.
        for b in cmd_part.iter_mut() {
            if *b >= b'a' && *b <= b'z' {
                *b -= 32;
            }
        }
        let cmd = std::str::from_utf8(cmd_part).map_err(|_| Error::LineMalformed)?;

        // Percent-decode args in-place if present (skip the space delimiter).
        let args = if rest.len() > 1 {
            Some(percent::decode_in_place(&mut rest[1..])?)
        } else {
            None
        };

        tracing::debug!(command = cmd, args = ?args, "parsed request");

        Ok(Some(Self { command: cmd, args }))
    }

    /// The command name, uppercased.
    pub fn command(&self) -> &str {
        self.command
    }

    /// The percent-decoded arguments, or `None` if no arguments were given.
    pub fn args(&self) -> Option<&str> {
        self.args
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: tests use bare lines (no LF/CRLF) because LineReader strips
    // line terminators before passing to Request::parse.

    #[test]
    fn parse_command_only() {
        let mut line = b"GETPIN".to_vec();
        let req = Request::parse(&mut line).unwrap().unwrap();
        assert_eq!(req.command(), "GETPIN");
        assert_eq!(req.args(), None);
    }

    #[test]
    fn parse_command_with_args() {
        let mut line = b"SETDESC Enter passphrase".to_vec();
        let req = Request::parse(&mut line).unwrap().unwrap();
        assert_eq!(req.command(), "SETDESC");
        assert_eq!(req.args(), Some("Enter passphrase"));
    }

    #[test]
    fn parse_lowercase_uppercased() {
        let mut line = b"getpin".to_vec();
        let req = Request::parse(&mut line).unwrap().unwrap();
        assert_eq!(req.command(), "GETPIN");
    }

    #[test]
    fn parse_percent_decoded_args() {
        let mut line = b"SETDESC hello%20world".to_vec();
        let req = Request::parse(&mut line).unwrap().unwrap();
        assert_eq!(req.args(), Some("hello world"));
    }

    #[test]
    fn parse_comment_line() {
        let mut line = b"# this is a comment".to_vec();
        let req = Request::parse(&mut line).unwrap().unwrap();
        assert_eq!(req.command(), "#");
        assert_eq!(req.args(), Some("this is a comment"));
    }

    #[test]
    fn parse_empty_line() {
        let mut line = b"".to_vec();
        assert!(Request::parse(&mut line).unwrap().is_none());
    }

    #[test]
    fn parse_option_with_equals() {
        let mut line = b"OPTION display=:0".to_vec();
        let req = Request::parse(&mut line).unwrap().unwrap();
        assert_eq!(req.command(), "OPTION");
        assert_eq!(req.args(), Some("display=:0"));
    }

    #[test]
    fn parse_confirm_one_button() {
        let mut line = b"CONFIRM --one-button".to_vec();
        let req = Request::parse(&mut line).unwrap().unwrap();
        assert_eq!(req.command(), "CONFIRM");
        assert_eq!(req.args(), Some("--one-button"));
    }

    #[test]
    fn parse_malformed_non_utf8() {
        let mut line = vec![0xFF, 0xFE];
        assert!(Request::parse(&mut line).is_err());
    }
}
