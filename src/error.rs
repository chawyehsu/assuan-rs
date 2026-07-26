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
    // -- General (libgpg-error range 0–200) --
    /// Success.
    pub const SUCCESS: Self = Self(0);
    /// General error.
    pub const GENERAL: Self = Self(1);
    /// Bad passphrase.
    pub const BAD_PASSPHRASE: Self = Self(11);
    /// Cannot open keyring.
    pub const KEYRING_OPEN: Self = Self(13);
    /// Invalid passphrase.
    pub const INV_PASSPHRASE: Self = Self(31);
    /// Invalid keyring.
    pub const INV_KEYRING: Self = Self(34);
    /// Unexpected error.
    pub const UNEXPECTED: Self = Self(38);
    /// Syntax error in URI.
    pub const BAD_URI: Self = Self(46);
    /// Invalid URI.
    pub const INV_URI: Self = Self(47);
    /// Network error.
    pub const NETWORK: Self = Self(48);
    /// Invalid value.
    pub const INV_VALUE: Self = Self(55);
    /// No data.
    pub const NO_DATA: Self = Self(58);
    /// Bug.
    pub const BUG: Self = Self(59);
    /// Not supported.
    pub const NOT_SUPPORTED: Self = Self(60);
    /// Operation timed out.
    pub const TIMEOUT: Self = Self(62);
    /// Internal error.
    pub const INTERNAL: Self = Self(63);
    /// Not implemented.
    pub const NOT_IMPLEMENTED: Self = Self(69);
    /// Invalid data.
    pub const INV_DATA: Self = Self(79);
    /// Unspecific Assuan server fault
    pub const ASSUAN_SERVER_FAULT: Self = Self(80);
    /// General Assuan error.
    pub const ASSUAN: Self = Self(81);
    /// No pinentry.
    pub const NO_PIN_ENTRY: Self = Self(85);
    /// Pinentry error.
    pub const PIN_ENTRY: Self = Self(86);
    /// Operation cancelled.
    pub const CANCELED: Self = Self(99);
    /// Not confirmed.
    pub const NOT_CONFIRMED: Self = Self(114);
    /// Configuration error.
    pub const CONFIGURATION: Self = Self(115);
    /// A locale function failed.
    pub const LOCALE_PROBLEM: Self = Self(166);
    /// Unknown option.
    pub const UNKNOWN_OPTION: Self = Self(174);
    /// Unknown command.
    pub const UNKNOWN_COMMAND: Self = Self(175);
    /// No passphrase given.
    pub const NO_PASSPHRASE: Self = Self(177);
    /// No PIN given.
    pub const NO_PIN: Self = Self(178);
    /// Operation fully cancelled.
    pub const FULLY_CANCELED: Self = Self(198);

    // -- Assuan-specific (libgpg-error range 257–281) --
    /// General IPC error.
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
    /// Reserved 272.
    const __RESERVED_272: Self = Self(272);
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
    pub const ASS_PARAMETER: Self = Self(280);
    /// Unknown IPC inquire.
    pub const ASS_UNKNOWN_INQUIRE: Self = Self(281);

    /// Screen or window too small.
    pub const WINDOW_TOO_SMALL: Self = Self(301);
    /// Screen or window too large.
    pub const WINDOW_TOO_LARGE: Self = Self(302);
    /// Required environment variable not set.
    pub const MISSING_ENVVAR: Self = Self(303);

    /// Unknown system error.
    pub const UNKNOWN_ERRNO: Self = Self(16382);
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Write the numeric code; the ERR line formatter will handle the rest.
        write!(f, "{}", self.0)
    }
}

/// Errors that can occur when using the crate.
#[derive(Debug, thiserror::Error)]
#[error("ERR {code}{}", message.as_deref().map(|m| format!(" {m}")).unwrap_or_default())]
pub struct Error {
    /// The error code.
    pub code: ErrorCode,
    /// Optional error message.
    pub message: Option<String>,
}

impl Error {
    /// Create a new protocol-level error with the given code and message.
    ///
    /// ```
    /// use assuan::{Error, ErrorCode};
    ///
    /// let e = Error::new(ErrorCode::GENERAL, "something broke");
    /// ```
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: Some(message.into()),
        }
    }

    /// Create a protocol error from an I/O error.
    ///
    /// Converts a `std::io::Error` into an `Error` with code
    /// `ErrorCode::GENERAL` and a message prefixed with "I/O: ".
    pub(crate) fn from_io(e: std::io::Error) -> Self {
        Self::new(ErrorCode::GENERAL, format!("I/O: {e}"))
    }
}

impl From<ErrorCode> for Error {
    fn from(code: ErrorCode) -> Self {
        Self {
            code,
            message: None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::from_io(e)
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
        assert!(matches!(
            e,
            Error {
                code: ErrorCode::GENERAL,
                message: Some(s)
            } if s == "oops"
        ));
    }

    #[test]
    fn error_new_with_string() {
        let e = Error::new(ErrorCode::ASS_UNKNOWN_CMD, "bad cmd".to_string());
        assert!(matches!(
            e,
            Error {
                code: ErrorCode::ASS_UNKNOWN_CMD,
                message: Some(s)
            } if s == "bad cmd"
        ));
    }

    #[test]
    fn error_from_code() {
        let e: Error = ErrorCode::CANCELED.into();
        assert!(matches!(
            e,
            Error {
                code: ErrorCode::CANCELED,
                message: None
            }
        ));
    }
}
