//! Integration tests for error type conversions.
//!
//! Tests the `ErrorCode → Error → Response::from(Error)` conversion paths
//! through the public API.

use assuan::{ErrorCode, Request, Response};

#[test]
fn error_code_to_error_conversion() {
    let e: assuan::Error = ErrorCode::CANCELED.into();
    assert!(matches!(
        e,
        assuan::Error::Err {
            code: ErrorCode::CANCELED,
            msg: None,
        }
    ));
}

#[test]
fn error_code_to_error_with_message() {
    let e = assuan::Error::new(ErrorCode::GENERAL, "something broke");
    assert!(matches!(
        e,
        assuan::Error::Err {
            code: ErrorCode::GENERAL,
            msg: Some(ref s),
        } if s == "something broke"
    ));
}

#[test]
fn error_to_response_err_variant() {
    let e = assuan::Error::Err {
        code: ErrorCode::ASS_UNKNOWN_CMD,
        msg: Some("unknown command".into()),
    };
    let resp: Response = e.into();
    assert_eq!(
        resp,
        Response::Err(ErrorCode::ASS_UNKNOWN_CMD, Some("unknown command".into()))
    );
}

#[test]
fn error_to_response_err_no_message() {
    let e: assuan::Error = ErrorCode::CANCELED.into();
    let resp: Response = e.into();
    assert_eq!(resp, Response::Err(ErrorCode::CANCELED, None));
}

#[test]
fn error_io_to_response_general() {
    let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke");
    let e = assuan::Error::Io(io_err);
    let resp: Response = e.into();
    assert_eq!(
        resp,
        Response::Err(ErrorCode::GENERAL, Some("I/O error".into()))
    );
}

#[test]
fn error_new_with_string() {
    let e = assuan::Error::new(ErrorCode::ASS_LINE_TOO_LONG, "too long".to_string());
    assert!(matches!(
        e,
        assuan::Error::Err {
            code: ErrorCode::ASS_LINE_TOO_LONG,
            msg: Some(ref s),
        } if s == "too long"
    ));
}

// ---------------------------------------------------------------------------
// ErrorCode constants
// ---------------------------------------------------------------------------

#[test]
fn error_code_display_values() {
    assert_eq!(ErrorCode::SUCCESS.to_string(), "0");
    assert_eq!(ErrorCode::GENERAL.to_string(), "1");
    assert_eq!(ErrorCode::ASS_GENERAL.to_string(), "257");
    assert_eq!(ErrorCode::ASS_UNKNOWN_CMD.to_string(), "275");
}

#[test]
fn error_code_equality() {
    assert_eq!(ErrorCode::GENERAL, ErrorCode(1));
    assert_eq!(ErrorCode::ASS_UNKNOWN_CMD, ErrorCode(275));
    assert_ne!(ErrorCode::GENERAL, ErrorCode(2));
}

#[test]
fn error_code_hash_consistency() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(ErrorCode::GENERAL);
    assert!(set.contains(&ErrorCode(1)));
}

// ---------------------------------------------------------------------------
// Parse errors through the wire
// ---------------------------------------------------------------------------

#[test]
fn request_parse_error_is_protocol_error() {
    let mut line = b"".to_vec();
    let result = Request::parse(&mut line);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        assuan::Error::Err {
            code: ErrorCode::ASS_INV_VALUE,
            ..
        }
    ));
}

#[test]
fn response_parse_error_is_protocol_error() {
    let mut line = b"".to_vec();
    let result = Response::parse(&mut line);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(
        err,
        assuan::Error::Err {
            code: ErrorCode::ASS_INV_VALUE,
            ..
        }
    ));
}

// ---------------------------------------------------------------------------
// Error → Response round-trip via wire
// ---------------------------------------------------------------------------

#[test]
fn error_to_response_roundtrip_through_wire() {
    let original = Response::err(ErrorCode::ASS_UNKNOWN_CMD, Some("bad cmd".into()));
    let mut buf = Vec::new();
    original.write_to(&mut buf).unwrap();
    // Strip trailing \n — LineReader does this before calling parse.
    buf.pop();
    let parsed = Response::parse(&mut buf).unwrap();
    assert_eq!(parsed, original);
}

#[test]
fn error_code_custom_value_roundtrip() {
    // Custom error code not in the named constants.
    let resp = Response::err(ErrorCode(9999), Some("custom".into()));
    let mut buf = Vec::new();
    resp.write_to(&mut buf).unwrap();
    // Strip trailing \n — LineReader does this before calling parse.
    buf.pop();
    let parsed = Response::parse(&mut buf).unwrap();
    assert_eq!(
        parsed,
        Response::Err(ErrorCode(9999), Some("custom".into()))
    );
}
