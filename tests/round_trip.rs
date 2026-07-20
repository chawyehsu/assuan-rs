//! Integration tests for Request/Response wire round-trips and encoding.
//!
//! Tests the serialize → parse contract through actual byte streams,
//! percent-encoded data surviving the full path, and line size limit
//! enforcement via the public API.

use assuan::{ErrorCode, Request, Response};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Serialize a request, then parse it back.
/// Strips the trailing `\n` that `write_to` adds, matching how `LineReader`
/// delivers lines to the parser.
fn roundtrip_request(req: &Request) -> Request {
    let mut buf = Vec::new();
    req.write_to(&mut buf).unwrap();
    // Strip trailing \n — LineReader does this before calling parse.
    assert_eq!(buf.last(), Some(&b'\n'));
    buf.pop();
    Request::parse(&mut buf).unwrap()
}

/// Serialize a response, then parse it back.
/// Strips the trailing `\n` that `write_to` adds, matching how `LineReader`
/// delivers lines to the parser.
fn roundtrip_response(resp: &Response) -> Response {
    let mut buf = Vec::new();
    resp.write_to(&mut buf).unwrap();
    // Strip trailing \n — LineReader does this before calling parse.
    assert_eq!(buf.last(), Some(&b'\n'));
    buf.pop();
    Response::parse(&mut buf).unwrap()
}

// ---------------------------------------------------------------------------
// Request round-trips
// ---------------------------------------------------------------------------

#[test]
fn request_roundtrip_command_no_args() {
    let req = Request::Command {
        name: "GETPIN".into(),
        args: None,
    };
    assert_eq!(roundtrip_request(&req), req);
}

#[test]
fn request_roundtrip_command_with_args() {
    let req = Request::Command {
        name: "SETDESC".into(),
        args: Some("Enter passphrase".into()),
    };
    assert_eq!(roundtrip_request(&req), req);
}

#[test]
fn request_roundtrip_command_with_percent_sensitive_args() {
    let req = Request::Command {
        name: "SETDESC".into(),
        args: Some("hello world 100%".into()),
    };
    assert_eq!(roundtrip_request(&req), req);
}

#[test]
fn request_roundtrip_bye() {
    assert_eq!(roundtrip_request(&Request::Bye), Request::Bye);
}

#[test]
fn request_roundtrip_reset() {
    assert_eq!(roundtrip_request(&Request::Reset), Request::Reset);
}

#[test]
fn request_roundtrip_nop() {
    assert_eq!(roundtrip_request(&Request::Nop), Request::Nop);
}

#[test]
fn request_roundtrip_end() {
    assert_eq!(roundtrip_request(&Request::End), Request::End);
}

#[test]
fn request_roundtrip_cancel() {
    assert_eq!(roundtrip_request(&Request::Cancel), Request::Cancel);
}

#[test]
fn request_roundtrip_help() {
    assert_eq!(roundtrip_request(&Request::Help), Request::Help);
}

#[test]
fn request_roundtrip_quit() {
    assert_eq!(roundtrip_request(&Request::Quit), Request::Quit);
}

#[test]
fn request_roundtrip_auth() {
    assert_eq!(roundtrip_request(&Request::Auth), Request::Auth);
}

#[test]
fn request_roundtrip_option_with_value() {
    let req = Request::Option {
        key: "display".into(),
        value: ":0".into(),
    };
    assert_eq!(roundtrip_request(&req), req);
}

#[test]
fn request_roundtrip_option_no_value() {
    let req = Request::Option {
        key: "ttyname".into(),
        value: "".into(),
    };
    assert_eq!(roundtrip_request(&req), req);
}

#[test]
fn request_roundtrip_comment() {
    let req = Request::Comment("this is a comment".into());
    assert_eq!(roundtrip_request(&req), req);
}

#[test]
fn request_roundtrip_data_simple() {
    let req = Request::Data(b"hello".to_vec());
    assert_eq!(roundtrip_request(&req), req);
}

#[test]
fn request_roundtrip_data_with_newline() {
    let req = Request::Data(b"line1\nline2".to_vec());
    assert_eq!(roundtrip_request(&req), req);
}

#[test]
fn request_roundtrip_data_with_percent() {
    let req = Request::Data(b"100%".to_vec());
    assert_eq!(roundtrip_request(&req), req);
}

#[test]
fn request_roundtrip_data_with_non_ascii() {
    // \x80 and \xFF are not valid UTF-8, so Request::parse will reject them.
    // Use control bytes that are valid single-byte UTF-8 instead.
    let req = Request::Data(vec![0x00, 0x01, 0x02, 0x1F]);
    assert_eq!(roundtrip_request(&req), req);
}

// ---------------------------------------------------------------------------
// Response round-trips
// ---------------------------------------------------------------------------

#[test]
fn response_roundtrip_ok_no_message() {
    assert_eq!(roundtrip_response(&Response::OK), Response::Ok(None));
}

#[test]
fn response_roundtrip_ok_with_message() {
    let resp = Response::ok("protocol version 1");
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_err_no_message() {
    let resp = Response::err(ErrorCode::GENERAL, None);
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_err_with_message() {
    let resp = Response::err(ErrorCode::ASS_UNKNOWN_CMD, Some("unknown".into()));
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_status_no_value() {
    let resp = Response::status("PASSWORD_FROM_CACHE", "");
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_status_with_value() {
    let resp = Response::status("BUTTON_INFO", "close");
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_data_simple() {
    let resp = Response::data(b"hello".to_vec());
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_data_with_newline() {
    let resp = Response::data(b"line1\nline2".to_vec());
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_data_with_percent() {
    let resp = Response::data(b"100%".to_vec());
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_data_empty() {
    let resp = Response::data(vec![]);
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_inquire_no_params() {
    let resp = Response::inquire("PASSPHRASE", "");
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_inquire_with_params() {
    let resp = Response::inquire("PASSPHRASE", "key=abc");
    assert_eq!(roundtrip_response(&resp), resp);
}

#[test]
fn response_roundtrip_comment() {
    let resp = Response::Comment("debug info".into());
    assert_eq!(roundtrip_response(&resp), resp);
}

// ---------------------------------------------------------------------------
// Line size limit enforcement
// ---------------------------------------------------------------------------

#[test]
fn request_write_line_too_long() {
    // Command with args that exceed MAX_LINE_SIZE (1000 bytes).
    let long_args = "x".repeat(1000);
    let req = Request::Command {
        name: "CMD".into(),
        args: Some(long_args),
    };
    let mut buf = Vec::new();
    let result = req.write_to(&mut buf);
    assert!(matches!(
        result,
        Err(assuan::Error::Err {
            code: ErrorCode::ASS_LINE_TOO_LONG,
            ..
        })
    ));
}

#[test]
fn request_write_data_too_long() {
    // Data that encodes to > 998 bytes (1000 - "D " prefix - \n).
    // Each \n encodes to 3 bytes (%0A), so 334 newlines → 1002 encoded bytes.
    let data = vec![b'\n'; 334];
    let req = Request::Data(data);
    let mut buf = Vec::new();
    let result = req.write_to(&mut buf);
    assert!(matches!(
        result,
        Err(assuan::Error::Err {
            code: ErrorCode::ASS_LINE_TOO_LONG,
            ..
        })
    ));
}

#[test]
fn response_write_ok_too_long() {
    let msg = "x".repeat(998);
    let resp = Response::ok(msg);
    let mut buf = Vec::new();
    let result = resp.write_to(&mut buf);
    assert!(matches!(
        result,
        Err(assuan::Error::Err {
            code: ErrorCode::ASS_LINE_TOO_LONG,
            ..
        })
    ));
}

#[test]
fn response_write_err_too_long() {
    // "ERR 1 " is 6 bytes, so message of 995 bytes → 6 + 995 + 1(\n) = 1002 > 1000.
    let msg = "x".repeat(995);
    let resp = Response::err(ErrorCode::GENERAL, Some(msg));
    let mut buf = Vec::new();
    let result = resp.write_to(&mut buf);
    assert!(matches!(
        result,
        Err(assuan::Error::Err {
            code: ErrorCode::ASS_LINE_TOO_LONG,
            ..
        })
    ));
}

#[test]
fn response_write_data_too_long() {
    let data = vec![b'\n'; 334];
    let resp = Response::data(data);
    let mut buf = Vec::new();
    let result = resp.write_to(&mut buf);
    assert!(matches!(
        result,
        Err(assuan::Error::Err {
            code: ErrorCode::ASS_LINE_TOO_LONG,
            ..
        })
    ));
}

#[test]
fn response_write_status_too_long() {
    let kw = "K".repeat(500);
    let val = "V".repeat(501);
    let resp = Response::status(kw, val);
    let mut buf = Vec::new();
    let result = resp.write_to(&mut buf);
    assert!(matches!(
        result,
        Err(assuan::Error::Err {
            code: ErrorCode::ASS_LINE_TOO_LONG,
            ..
        })
    ));
}

#[test]
fn response_write_inquire_too_long() {
    let kw = "K".repeat(500);
    let params = "P".repeat(501);
    let resp = Response::inquire(kw, params);
    let mut buf = Vec::new();
    let result = resp.write_to(&mut buf);
    assert!(matches!(
        result,
        Err(assuan::Error::Err {
            code: ErrorCode::ASS_LINE_TOO_LONG,
            ..
        })
    ));
}

// ---------------------------------------------------------------------------
// Parse errors
// ---------------------------------------------------------------------------

#[test]
fn request_parse_empty_line_is_err() {
    let mut line = b"".to_vec();
    assert!(Request::parse(&mut line).is_err());
}

#[test]
fn response_parse_empty_line_is_err() {
    let mut line = b"".to_vec();
    assert!(Response::parse(&mut line).is_err());
}

#[test]
fn response_parse_bare_d_is_err() {
    let mut line = b"D".to_vec();
    assert!(Response::parse(&mut line).is_err());
}

#[test]
fn response_parse_unknown_prefix_is_err() {
    let mut line = b"FOOBAR".to_vec();
    assert!(Response::parse(&mut line).is_err());
}
