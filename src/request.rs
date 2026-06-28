//! Assuan request types and parsing.
//!
//! The [`Request`] enum represents all client-side request lines in the Assuan
//! protocol, covering both protocol-level commands and application commands.

use std::fmt;
use std::io::Write;

use crate::MAX_LINE_SIZE;
use crate::Error;
use crate::percent;

/// An Assuan client request.
///
/// Covers protocol-level commands (defined by the spec) and application
/// commands (defined by the server). All data is owned.
///
/// # Parsing
///
/// Use [`Request::parse`] to convert a raw line (from [`LineReader`]) into a
/// typed request:
///
/// ```
/// use assuan::Request;
///
/// let mut line = b"GETPIN".to_vec();
/// let req = Request::parse(&mut line).unwrap();
/// assert_eq!(req, Request::Command { name: "GETPIN".into(), args: None });
/// ```
///
/// # Writing
///
/// Use [`Request::write_to`] to serialize a request to the wire:
///
/// ```
/// use assuan::Request;
///
/// let req = Request::Bye;
/// let mut buf = Vec::new();
/// req.write_to(&mut buf).unwrap();
/// assert_eq!(buf, b"BYE\n");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    // -- Protocol-level commands (defined by the Assuan spec) --

    /// Close the connection. Server responds with OK.
    Bye,

    /// Reset the connection but not authentication.
    Reset,

    /// No operation. Server responds with OK.
    Nop,

    /// End of data stream (client responds to INQUIRE).
    End,

    /// Cancel the current operation (client responds to INQUIRE).
    Cancel,

    /// Set a connection option.
    Option {
        /// The option name.
        key: String,
        /// The option value (empty for boolean flags).
        value: String,
    },

    /// List available commands.
    Help,

    /// Reserved for future extensions.
    Quit,

    /// Reserved for future extensions.
    Auth,

    // -- Wire-level --

    /// Data line (`D <data>`) — used during INQUIRE responses.
    Data(Vec<u8>),

    // -- Application command fallback --

    /// Comment line (`# ...`) — ignored per the Assuan spec.
    Comment(String),

    /// An application-level command with name and optional arguments.
    Command {
        /// The command name (uppercased).
        name: String,
        /// The percent-decoded arguments, if any.
        args: Option<String>,
    },
}

impl Request {
    /// Parse a raw line into a request.
    ///
    /// The input should come from [`LineReader::read`](crate::LineReader::read),
    /// which already strips trailing LF/CRLF. The command name is uppercased
    /// in-place; arguments are percent-decoded.
    ///
    /// Returns `Err` if the line is empty or not valid UTF-8.
    pub fn parse(line: &mut [u8]) -> Result<Self, Error> {
        // Strip trailing LF/CRLF if present (defensive — LineReader does this,
        // but roundtrip tests pass raw bytes).
        let line = if line.last() == Some(&b'\n') {
            let len = line.len() - 1;
            if len > 0 && line[len - 1] == b'\r' {
                &mut line[..len - 1]
            } else {
                &mut line[..len]
            }
        } else {
            line
        };

        if line.is_empty() {
            return Err(Error::LineMalformed);
        }

        // Split into command and args at the first space.
        let space_pos = line.iter().position(|&b| b == b' ');
        let cmd_end = space_pos.unwrap_or(line.len());

        let (cmd_part, rest) = line.split_at_mut(cmd_end);

        // Uppercase the command name in-place.
        for b in cmd_part.iter_mut() {
            if b.is_ascii_lowercase() {
                *b -= 32;
            }
        }
        let cmd = std::str::from_utf8(cmd_part).map_err(|_| Error::LineMalformed)?;

        // Percent-decode args if present (skip the space delimiter).
        let args = if rest.len() > 1 {
            Some(percent::decode_in_place(&mut rest[1..])?.to_string())
        } else {
            None
        };

        tracing::debug!(command = cmd, args = ?args, "parsed request");

        // Comment lines (# ...) are ignored per the Assuan spec.
        if cmd == "#" {
            let comment = args.unwrap_or_default();
            return Ok(Request::Comment(comment));
        }

        // Map to the appropriate variant.
        match cmd {
            // Protocol-level commands
            "BYE" => Ok(Request::Bye),
            "RESET" => Ok(Request::Reset),
            "NOP" => Ok(Request::Nop),
            "END" => Ok(Request::End),
            "CAN" => Ok(Request::Cancel),
            "HELP" => Ok(Request::Help),
            "QUIT" => Ok(Request::Quit),
            "AUTH" => Ok(Request::Auth),
            "OPTION" => {
                let (key, value) = parse_option_args(args.as_deref().unwrap_or(""));
                Ok(Request::Option { key, value })
            }
            // Data line
            "D" => {
                let data = args.unwrap_or_default();
                // Percent-decode the data payload.
                let mut data_bytes = data.into_bytes();
                let decoded = percent::decode_in_place(&mut data_bytes)?;
                Ok(Request::Data(decoded.as_bytes().to_vec()))
            }
            // Application command fallback
            _ => Ok(Request::Command {
                name: cmd.to_string(),
                args,
            }),
        }
    }

    /// Write this request to the given writer.
    ///
    /// Returns the number of bytes written. Enforces [`MAX_LINE_SIZE`].
    pub fn write_to<W: Write>(&self, w: &mut W) -> Result<usize, Error> {
        match self {
            Request::Bye => write_line(w, b"BYE"),
            Request::Reset => write_line(w, b"RESET"),
            Request::Nop => write_line(w, b"NOP"),
            Request::End => write_line(w, b"END"),
            Request::Cancel => write_line(w, b"CAN"),
            Request::Help => write_line(w, b"HELP"),
            Request::Quit => write_line(w, b"QUIT"),
            Request::Auth => write_line(w, b"AUTH"),
            Request::Option { key, value } => {
                if value.is_empty() {
                    let line = format!("OPTION {key}");
                    write_line(w, line.as_bytes())
                } else {
                    let line = format!("OPTION {key}={value}");
                    write_line(w, line.as_bytes())
                }
            }
            Request::Data(data) => write_data(w, data),
            Request::Comment(s) => {
                let line = format!("# {s}");
                write_line(w, line.as_bytes())
            }
            Request::Command { name, args } => {
                match args {
                    None => write_line(w, name.as_bytes()),
                    Some(args) => {
                        let encoded_len = percent::encoded_len(args.len());
                        let mut buf = Vec::with_capacity(name.len() + 1 + encoded_len);
                        buf.extend_from_slice(name.as_bytes());
                        buf.push(b' ');
                        let mut enc_buf = vec![0u8; encoded_len];
                        let n = percent::encode(args.as_bytes(), &mut enc_buf);
                        buf.extend_from_slice(&enc_buf[..n]);
                        write_line(w, &buf)
                    }
                }
            }
        }
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Request::Bye => write!(f, "BYE"),
            Request::Reset => write!(f, "RESET"),
            Request::Nop => write!(f, "NOP"),
            Request::End => write!(f, "END"),
            Request::Cancel => write!(f, "CAN"),
            Request::Help => write!(f, "HELP"),
            Request::Quit => write!(f, "QUIT"),
            Request::Auth => write!(f, "AUTH"),
            Request::Option { key, value } => {
                if value.is_empty() {
                    write!(f, "OPTION {key}")
                } else {
                    write!(f, "OPTION {key}={value}")
                }
            }
            Request::Data(_) => write!(f, "D <data>"),
            Request::Comment(s) => write!(f, "# {s}"),
            Request::Command { name, args } => match args {
                None => write!(f, "{name}"),
                Some(args) => write!(f, "{name} {args}"),
            },
        }
    }
}

/// Parse OPTION arguments: `name [= value]`
fn parse_option_args(args: &str) -> (String, String) {
    if args.is_empty() {
        return (String::new(), String::new());
    }
    match args.find('=') {
        Some(pos) => {
            let key = args[..pos].trim().to_string();
            let value = args[pos + 1..].trim().to_string();
            (key, value)
        }
        None => {
            let key = args.trim().to_string();
            (key, String::new())
        }
    }
}

/// Write a line with `\n` terminator.
fn write_line<W: Write>(w: &mut W, line: &[u8]) -> Result<usize, Error> {
    let total = line.len() + 1;
    if total > MAX_LINE_SIZE {
        return Err(Error::LineTooLong);
    }
    w.write_all(line).map_err(Error::Io)?;
    w.write_all(b"\n").map_err(Error::Io)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    // -- parse --

    #[test]
    fn parse_command_only() {
        let mut line = b"GETPIN".to_vec();
        let req = Request::parse(&mut line).unwrap();
        assert_eq!(req, Request::Command { name: "GETPIN".into(), args: None });
    }

    #[test]
    fn parse_command_with_args() {
        let mut line = b"SETDESC Enter passphrase".to_vec();
        let req = Request::parse(&mut line).unwrap();
        assert_eq!(req, Request::Command {
            name: "SETDESC".into(),
            args: Some("Enter passphrase".into()),
        });
    }

    #[test]
    fn parse_lowercase_uppercased() {
        let mut line = b"getpin".to_vec();
        let req = Request::parse(&mut line).unwrap();
        assert_eq!(req, Request::Command { name: "GETPIN".into(), args: None });
    }

    #[test]
    fn parse_percent_decoded_args() {
        let mut line = b"SETDESC hello%20world".to_vec();
        let req = Request::parse(&mut line).unwrap();
        assert_eq!(req, Request::Command {
            name: "SETDESC".into(),
            args: Some("hello world".into()),
        });
    }

    #[test]
    fn parse_comment_line() {
        let mut line = b"# this is a comment".to_vec();
        let req = Request::parse(&mut line).unwrap();
        assert_eq!(req, Request::Comment("this is a comment".into()));
    }

    #[test]
    fn parse_empty_line() {
        let mut line = b"".to_vec();
        assert!(Request::parse(&mut line).is_err());
    }

    #[test]
    fn parse_option_with_equals() {
        let mut line = b"OPTION display=:0".to_vec();
        let req = Request::parse(&mut line).unwrap();
        assert_eq!(req, Request::Option {
            key: "display".into(),
            value: ":0".into(),
        });
    }

    #[test]
    fn parse_bye() {
        let mut line = b"BYE".to_vec();
        assert_eq!(Request::parse(&mut line).unwrap(), Request::Bye);
    }

    #[test]
    fn parse_reset() {
        let mut line = b"RESET".to_vec();
        assert_eq!(Request::parse(&mut line).unwrap(), Request::Reset);
    }

    #[test]
    fn parse_nop() {
        let mut line = b"NOP".to_vec();
        assert_eq!(Request::parse(&mut line).unwrap(), Request::Nop);
    }

    #[test]
    fn parse_end() {
        let mut line = b"END".to_vec();
        assert_eq!(Request::parse(&mut line).unwrap(), Request::End);
    }

    #[test]
    fn parse_cancel() {
        let mut line = b"CAN".to_vec();
        assert_eq!(Request::parse(&mut line).unwrap(), Request::Cancel);
    }

    #[test]
    fn parse_data() {
        let mut line = b"D hello".to_vec();
        let req = Request::parse(&mut line).unwrap();
        assert_eq!(req, Request::Data(b"hello".to_vec()));
    }

    #[test]
    fn parse_data_encoded() {
        let mut line = b"D hello%0Aworld".to_vec();
        let req = Request::parse(&mut line).unwrap();
        assert_eq!(req, Request::Data(b"hello\nworld".to_vec()));
    }

    // -- write_to --

    #[test]
    fn write_bye() {
        let mut buf = Vec::new();
        Request::Bye.write_to(&mut buf).unwrap();
        assert_eq!(buf, b"BYE\n");
    }

    #[test]
    fn write_command_no_args() {
        let mut buf = Vec::new();
        Request::Command { name: "GETPIN".into(), args: None }
            .write_to(&mut buf)
            .unwrap();
        assert_eq!(buf, b"GETPIN\n");
    }

    #[test]
    fn write_command_with_args() {
        let mut buf = Vec::new();
        Request::Command { name: "SETDESC".into(), args: Some("hello world".into()) }
            .write_to(&mut buf)
            .unwrap();
        // Spaces are NOT percent-encoded in command args (only in D data lines).
        assert_eq!(buf, b"SETDESC hello world\n");
    }

    #[test]
    fn write_data_line() {
        let mut buf = Vec::new();
        Request::Data(b"hello".to_vec()).write_to(&mut buf).unwrap();
        assert_eq!(buf, b"D hello\n");
    }

    #[test]
    fn write_option() {
        let mut buf = Vec::new();
        Request::Option { key: "display".into(), value: ":0".into() }
            .write_to(&mut buf)
            .unwrap();
        assert_eq!(buf, b"OPTION display=:0\n");
    }

    // -- round-trip --

    #[test]
    fn roundtrip_bye() {
        let mut buf = Vec::new();
        Request::Bye.write_to(&mut buf).unwrap();
        let req = Request::parse(&mut buf).unwrap();
        assert_eq!(req, Request::Bye);
    }

    #[test]
    fn roundtrip_command() {
        let original = Request::Command {
            name: "SETDESC".into(),
            args: Some("Enter passphrase".into()),
        };
        let mut buf = Vec::new();
        original.write_to(&mut buf).unwrap();
        let parsed = Request::parse(&mut buf).unwrap();
        assert_eq!(parsed, original);
    }

    // -- Display --

    #[test]
    fn display_bye() {
        assert_eq!(Request::Bye.to_string(), "BYE");
    }

    #[test]
    fn display_command() {
        assert_eq!(
            Request::Command { name: "GETPIN".into(), args: None }.to_string(),
            "GETPIN"
        );
        assert_eq!(
            Request::Command { name: "SETDESC".into(), args: Some("hello".into()) }.to_string(),
            "SETDESC hello"
        );
    }
}
