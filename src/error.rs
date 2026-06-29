//! Assuan protocol error types.

use std::fmt;

/// Error code used in Assuan `ERR` responses.
///
/// A `u32` newtype for interoperability with libgpg-error. Named constants are
/// provided for the commonly used subset; consumers can construct arbitrary
/// codes via `ErrorCode(n)` if needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode(pub u32);

impl ErrorCode {
    // -- General codes (libgpg-error range 0–100) --
    /// Success.
    pub const SUCCESS: Self = Self(0);
    /// General error.
    pub const GENERAL: Self = Self(1);
    /// Invalid parameter / bad parameter.
    pub const INV_PARAMETER: Self = Self(67);
    /// Internal error.
    pub const INTERNAL: Self = Self(63);
    /// Not implemented.
    pub const NOT_IMPLEMENTED: Self = Self(69);
    /// Out of memory.
    pub const ENOMEM: Self = Self(86);
    /// Operation cancelled.
    pub const CANCELED: Self = Self(99);
    /// Not confirmed.
    pub const NOT_CONFIRMED: Self = Self(114);
    /// Locale problem.
    pub const LOCALE_PROBLEM: Self = Self(160);
    /// No PIN given.
    pub const NO_PIN: Self = Self(178);

    // -- Assuan-specific codes (libgpg-error range 257–281) --
    /// General IPC (Assuan) error.
    pub const ASS_GENERAL: Self = Self(257);
    /// IPC accept call failed.
    pub const ASS_ACCEPT_FAILED: Self = Self(258);
    /// IPC connect call failed.
    pub const ASS_CONNECT_FAILED: Self = Self(259);
    /// Invalid IPC response.
    pub const ASS_INV_RESPONSE: Self = Self(260);
    /// Invalid value passed to IPC.
    pub const ASS_INV_VALUE: Self = Self(261);
    /// Incomplete line passed to IPC.
    pub const ASS_INCOMPLETE_LINE: Self = Self(262);
    /// Line passed to IPC too long.
    pub const ASS_LINE_TOO_LONG: Self = Self(263);
    /// Nested IPC commands.
    pub const ASS_NESTED_COMMANDS: Self = Self(264);
    /// No data callback in IPC.
    pub const ASS_NO_DATA_CB: Self = Self(265);
    /// No inquire callback in IPC.
    pub const ASS_NO_INQUIRE_CB: Self = Self(266);
    /// Not an IPC server.
    pub const ASS_NOT_A_SERVER: Self = Self(267);
    /// Not an IPC client.
    pub const ASS_NOT_A_CLIENT: Self = Self(268);
    /// Problem starting IPC server.
    pub const ASS_SERVER_START: Self = Self(269);
    /// IPC read error.
    pub const ASS_READ_ERROR: Self = Self(270);
    /// IPC write error.
    pub const ASS_WRITE_ERROR: Self = Self(271);
    /// Too much data for IPC layer.
    pub const ASS_TOO_MUCH_DATA: Self = Self(273);
    /// Unexpected IPC command.
    pub const ASS_UNEXPECTED_CMD: Self = Self(274);
    /// Unknown IPC command.
    pub const ASS_UNKNOWN_CMD: Self = Self(275);
    /// IPC syntax error.
    pub const ASS_SYNTAX: Self = Self(276);
    /// IPC call has been cancelled.
    pub const ASS_CANCELED: Self = Self(277);
    /// No input source for IPC.
    pub const ASS_NO_INPUT: Self = Self(278);
    /// No output source for IPC.
    pub const ASS_NO_OUTPUT: Self = Self(279);
    /// IPC parameter error.
    pub const ASS_PARAMETER_ERROR: Self = Self(280);
    /// Unknown IPC inquire.
    pub const ASS_UNKNOWN_INQUIRE: Self = Self(281);
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write the numeric code; the ERR line formatter will handle the rest.
        write!(f, "{}", self.0)
    }
}

/// Errors that can occur when using the crate.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O error occurred.
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),

    /// A protocol-level error to be sent as an `ERR` response.
    #[error("ERR {code}{}", msg.as_deref().unwrap_or(""))]
    Err {
        /// The error code.
        code: ErrorCode,
        /// Optional error message.
        msg: Option<String>,
    },
}

impl Error {
    /// Create a new protocol-level error with the given code and message.
    ///
    /// ```
    /// use assuan::{Error, ErrorCode};
    ///
    /// let e = Error::new(ErrorCode::GENERAL, "something broke");
    /// ```
    pub fn new(code: ErrorCode, msg: impl Into<String>) -> Self {
        Error::Err {
            code,
            msg: Some(msg.into()),
        }
    }
}

impl From<ErrorCode> for Error {
    fn from(code: ErrorCode) -> Self {
        Error::Err { code, msg: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_display() {
        assert_eq!(ErrorCode::GENERAL.to_string(), "1");
        assert_eq!(ErrorCode::ASS_UNKNOWN_CMD.to_string(), "275");
    }

    #[test]
    fn error_code_equality() {
        assert_eq!(ErrorCode::GENERAL, ErrorCode(1));
        assert_ne!(ErrorCode::GENERAL, ErrorCode(2));
    }

    #[test]
    fn error_new_with_str() {
        let e = Error::new(ErrorCode::GENERAL, "oops");
        assert!(matches!(e, Error::Err { code: ErrorCode::GENERAL, msg: Some(s) } if s == "oops"));
    }

    #[test]
    fn error_new_with_string() {
        let e = Error::new(ErrorCode::ASS_UNKNOWN_CMD, "bad cmd".to_string());
        assert!(
            matches!(e, Error::Err { code: ErrorCode::ASS_UNKNOWN_CMD, msg: Some(s) } if s == "bad cmd")
        );
    }

    #[test]
    fn error_from_code() {
        let e: Error = ErrorCode::CANCELED.into();
        assert!(matches!(
            e,
            Error::Err {
                code: ErrorCode::CANCELED,
                msg: None
            }
        ));
    }
}
