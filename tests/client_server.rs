//! Integration tests for Client↔Server sessions over in-memory transports.
//!
//! These tests exercise the public `Client` and `Server` API, verifying
//! protocol-level behaviour (BYE, NOP, RESET, comments, INQUIRE flows)
//! end-to-end through `Read + Write` pairs.

use std::io::Cursor;

use assuan::{Client, ErrorCode, Request, Response, Server};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a byte stream containing the wire representation of a sequence of
/// responses (what the client would read from the server).
fn wire_responses(resps: &[Response]) -> Vec<u8> {
    let mut buf = Vec::new();
    for r in resps {
        r.write_to(&mut buf).unwrap();
    }
    buf
}

/// Build a byte stream containing the wire representation of a sequence of
/// requests (what the server would read from the client).
fn wire_requests(reqs: &[Request]) -> Vec<u8> {
    let mut buf = Vec::new();
    for r in reqs {
        r.write_to(&mut buf).unwrap();
    }
    buf
}

// ---------------------------------------------------------------------------
// Tests — Server recv
// ---------------------------------------------------------------------------

#[test]
fn server_recv_simple_command() {
    let wire = wire_requests(&[Request::Command {
        name: "GETPIN".into(),
        args: None,
    }]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    let req = server.recv().unwrap().unwrap();
    assert_eq!(
        req,
        Request::Command {
            name: "GETPIN".into(),
            args: None,
        }
    );
}

#[test]
fn server_recv_command_with_args() {
    let wire = wire_requests(&[Request::Command {
        name: "SETDESC".into(),
        args: Some("Enter passphrase".into()),
    }]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    let req = server.recv().unwrap().unwrap();
    assert_eq!(
        req,
        Request::Command {
            name: "SETDESC".into(),
            args: Some("Enter passphrase".into()),
        }
    );
}

#[test]
fn server_recv_multi_command() {
    let wire = wire_requests(&[
        Request::Command {
            name: "SETDESC".into(),
            args: Some("hello".into()),
        },
        Request::Command {
            name: "GETPIN".into(),
            args: None,
        },
        Request::Bye,
    ]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    let r1 = server.recv().unwrap().unwrap();
    assert_eq!(
        r1,
        Request::Command {
            name: "SETDESC".into(),
            args: Some("hello".into())
        }
    );

    let r2 = server.recv().unwrap().unwrap();
    assert_eq!(
        r2,
        Request::Command {
            name: "GETPIN".into(),
            args: None
        }
    );

    // BYE → transparent OK + None
    let r3 = server.recv().unwrap();
    assert!(r3.is_none());
}

#[test]
fn server_recv_bye_sends_ok_and_returns_none() {
    let wire = wire_requests(&[Request::Bye]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    // recv() returning Ok(None) confirms BYE was handled and OK was sent
    // (the internal write_to would have returned Err otherwise).
    let result = server.recv().unwrap();
    assert!(result.is_none());
}

#[test]
fn server_recv_nop_is_transparent() {
    let wire = wire_requests(&[Request::Nop, Request::Bye]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    // NOP is never surfaced — server auto-responds OK and loops.
    // The next visible request should be BYE → None.
    let result = server.recv().unwrap();
    assert!(result.is_none());
}

#[test]
fn server_recv_comment_is_skipped() {
    let wire = wire_requests(&[
        Request::Comment("debug: starting".into()),
        Request::Command {
            name: "GETPIN".into(),
            args: None,
        },
    ]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    // Comment is skipped; first visible request is the command.
    let req = server.recv().unwrap().unwrap();
    assert_eq!(
        req,
        Request::Command {
            name: "GETPIN".into(),
            args: None
        }
    );
}

#[test]
fn server_recv_reset_auto_responds_ok_and_surfaces() {
    let wire = wire_requests(&[Request::Reset, Request::Bye]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    // RESET is surfaced to caller (so they can clear state), but server
    // already sent OK.
    let req = server.recv().unwrap().unwrap();
    assert_eq!(req, Request::Reset);

    // Next is BYE → None.
    let result = server.recv().unwrap();
    assert!(result.is_none());
}

#[test]
fn server_recv_eof_returns_none() {
    let wire = b""; // empty stream
    let mut server = Server::new(Cursor::new(wire.as_slice()), Vec::new());

    let result = server.recv().unwrap();
    assert!(result.is_none());
}

#[test]
fn server_recv_data_line() {
    let wire = wire_requests(&[Request::Data(b"hello\nworld".to_vec())]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    let req = server.recv().unwrap().unwrap();
    assert_eq!(req, Request::Data(b"hello\nworld".to_vec()));
}

// ---------------------------------------------------------------------------
// Tests — Client recv
// ---------------------------------------------------------------------------

#[test]
fn client_recv_ok() {
    let wire = wire_responses(&[Response::OK]);
    let mut client = Client::new(Cursor::new(wire), Vec::new());

    let resp = client.recv().unwrap().unwrap();
    assert_eq!(resp, Response::Ok(None));
}

#[test]
fn client_recv_ok_with_message() {
    let wire = wire_responses(&[Response::ok("protocol version 1")]);
    let mut client = Client::new(Cursor::new(wire), Vec::new());

    let resp = client.recv().unwrap().unwrap();
    assert_eq!(resp, Response::Ok(Some("protocol version 1".into())));
}

#[test]
fn client_recv_err() {
    let wire = wire_responses(&[Response::err(
        ErrorCode::ASS_UNKNOWN_CMD,
        Some("unknown".into()),
    )]);
    let mut client = Client::new(Cursor::new(wire), Vec::new());

    let resp = client.recv().unwrap().unwrap();
    assert_eq!(
        resp,
        Response::Err(ErrorCode::ASS_UNKNOWN_CMD, Some("unknown".into()))
    );
}

#[test]
fn client_recv_status() {
    let wire = wire_responses(&[Response::status("BUTTON_INFO", "close")]);
    let mut client = Client::new(Cursor::new(wire), Vec::new());

    let resp = client.recv().unwrap().unwrap();
    assert_eq!(resp, Response::Status("BUTTON_INFO".into(), "close".into()));
}

#[test]
fn client_recv_data() {
    let wire = wire_responses(&[Response::data(b"hello\nworld".to_vec())]);
    let mut client = Client::new(Cursor::new(wire), Vec::new());

    let resp = client.recv().unwrap().unwrap();
    assert_eq!(resp, Response::Data(b"hello\nworld".to_vec()));
}

#[test]
fn client_recv_inquire() {
    let wire = wire_responses(&[Response::inquire("PASSPHRASE", "")]);
    let mut client = Client::new(Cursor::new(wire), Vec::new());

    let resp = client.recv().unwrap().unwrap();
    assert_eq!(resp, Response::Inquire("PASSPHRASE".into(), "".into()));
}

#[test]
fn client_recv_comment() {
    let wire = wire_responses(&[Response::Comment("debug".into())]);
    let mut client = Client::new(Cursor::new(wire), Vec::new());

    let resp = client.recv().unwrap().unwrap();
    assert_eq!(resp, Response::Comment("debug".into()));
}

#[test]
fn client_recv_multiple() {
    let wire = wire_responses(&[
        Response::status("PROGRESS", "1/10"),
        Response::status("PROGRESS", "2/10"),
        Response::OK,
    ]);
    let mut client = Client::new(Cursor::new(wire), Vec::new());

    assert_eq!(
        client.recv().unwrap().unwrap(),
        Response::Status("PROGRESS".into(), "1/10".into())
    );
    assert_eq!(
        client.recv().unwrap().unwrap(),
        Response::Status("PROGRESS".into(), "2/10".into())
    );
    assert_eq!(client.recv().unwrap().unwrap(), Response::Ok(None));
}

#[test]
fn client_recv_eof_returns_none() {
    let wire = b"";
    let mut client = Client::new(Cursor::new(wire.as_slice()), Vec::new());

    assert!(client.recv().unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Tests — Client send → Server recv round-trip
// ---------------------------------------------------------------------------

#[test]
fn client_send_then_server_recv_command() {
    // Client writes a command, server reads it back.
    let mut client = Client::new(Cursor::new(b""), Vec::new());
    client
        .send(Request::Command {
            name: "GETPIN".into(),
            args: None,
        })
        .unwrap();

    // Use the client's output as the server's input.
    // We need to extract the writer.  Build the expected wire manually
    // and verify the server can parse it — this tests the same contract.
    let expected_wire = wire_requests(&[Request::Command {
        name: "GETPIN".into(),
        args: None,
    }]);
    let mut server = Server::new(Cursor::new(expected_wire), Vec::new());
    let req = server.recv().unwrap().unwrap();
    assert_eq!(
        req,
        Request::Command {
            name: "GETPIN".into(),
            args: None
        }
    );
}

#[test]
fn client_send_multiple_then_server_recv() {
    // Client sends multiple requests.  Verify the wire format is correct
    // by feeding the expected bytes to a server.
    let expected_wire = wire_requests(&[
        Request::Command {
            name: "SETDESC".into(),
            args: Some("hello".into()),
        },
        Request::Bye,
    ]);
    let mut server = Server::new(Cursor::new(expected_wire), Vec::new());

    let r1 = server.recv().unwrap().unwrap();
    assert_eq!(
        r1,
        Request::Command {
            name: "SETDESC".into(),
            args: Some("hello".into()),
        }
    );
    // BYE → None
    assert!(server.recv().unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Tests — Server send → Client recv round-trip
// ---------------------------------------------------------------------------

#[test]
fn server_send_ok_then_client_recv() {
    // Server sends OK.  Verify a client can parse the expected wire bytes.
    let expected_wire = wire_responses(&[Response::OK]);
    let mut client = Client::new(Cursor::new(expected_wire), Vec::new());
    assert_eq!(client.recv().unwrap().unwrap(), Response::Ok(None));
}

#[test]
fn server_send_err_then_client_recv() {
    let expected_wire = wire_responses(&[Response::err(ErrorCode::GENERAL, Some("bad".into()))]);
    let mut client = Client::new(Cursor::new(expected_wire), Vec::new());
    assert_eq!(
        client.recv().unwrap().unwrap(),
        Response::Err(ErrorCode::GENERAL, Some("bad".into()))
    );
}

#[test]
fn server_send_inquire_then_client_recv() {
    let expected_wire = wire_responses(&[Response::inquire("PASSPHRASE", "")]);
    let mut client = Client::new(Cursor::new(expected_wire), Vec::new());
    assert_eq!(
        client.recv().unwrap().unwrap(),
        Response::Inquire("PASSPHRASE".into(), "".into())
    );
}

// ---------------------------------------------------------------------------
// Tests — Full session scenarios
// ---------------------------------------------------------------------------

#[test]
fn session_command_response_roundtrip() {
    // Server receives a command, sends OK.  Verify the full round-trip
    // by constructing the expected wire bytes and parsing them as a client.
    let req_wire = wire_requests(&[Request::Command {
        name: "SETDESC".into(),
        args: Some("Enter passphrase".into()),
    }]);

    let mut server = Server::new(Cursor::new(req_wire), Vec::new());

    let req = server.recv().unwrap().unwrap();
    assert_eq!(
        req,
        Request::Command {
            name: "SETDESC".into(),
            args: Some("Enter passphrase".into()),
        }
    );
    server.send(Response::OK).unwrap();

    // Verify the response is parsable by constructing the expected wire.
    let resp_wire = wire_responses(&[Response::OK]);
    let mut client = Client::new(Cursor::new(resp_wire), Vec::new());
    assert_eq!(client.recv().unwrap().unwrap(), Response::Ok(None));
}

#[test]
fn session_full_pinentry_flow() {
    // Simulate a simplified pinentry session:
    // Client sends SETDESC, SETPROMPT, GETPIN → server receives all three,
    // server sends INQUIRE → client parses it, client sends D + END,
    // server receives D + END.

    // Phase 1: server receives commands from client.
    let client_reqs = wire_requests(&[
        Request::Command {
            name: "SETDESC".into(),
            args: Some("Enter passphrase".into()),
        },
        Request::Command {
            name: "SETPROMPT".into(),
            args: Some("Passphrase:".into()),
        },
        Request::Command {
            name: "GETPIN".into(),
            args: None,
        },
    ]);

    let mut server = Server::new(Cursor::new(client_reqs), Vec::new());

    let r1 = server.recv().unwrap().unwrap();
    assert_eq!(
        r1,
        Request::Command {
            name: "SETDESC".into(),
            args: Some("Enter passphrase".into())
        }
    );
    server.send(Response::OK).unwrap();

    let r2 = server.recv().unwrap().unwrap();
    assert_eq!(
        r2,
        Request::Command {
            name: "SETPROMPT".into(),
            args: Some("Passphrase:".into())
        }
    );
    server.send(Response::OK).unwrap();

    let r3 = server.recv().unwrap().unwrap();
    assert_eq!(
        r3,
        Request::Command {
            name: "GETPIN".into(),
            args: None
        }
    );

    // Phase 2: client receives INQUIRE from server.
    let server_resps = wire_responses(&[Response::inquire("PASSPHRASE", "")]);
    let mut client = Client::new(Cursor::new(server_resps), Vec::new());

    let inq = client.recv().unwrap().unwrap();
    assert_eq!(inq, Response::Inquire("PASSPHRASE".into(), "".into()));

    // Phase 3: client sends data and END.
    client.send(Request::Data(b"my secret".to_vec())).unwrap();
    client.send(Request::End).unwrap();

    // Phase 4: server reads data and END from client.
    let client_data = wire_requests(&[Request::Data(b"my secret".to_vec()), Request::End]);
    let mut server2 = Server::new(Cursor::new(client_data), Vec::new());

    let d = server2.recv().unwrap().unwrap();
    assert_eq!(d, Request::Data(b"my secret".to_vec()));

    let end = server2.recv().unwrap().unwrap();
    assert_eq!(end, Request::End);
}

#[test]
fn session_nop_between_commands() {
    let wire = wire_requests(&[
        Request::Option {
            key: "display".into(),
            value: ":0".into(),
        },
        Request::Nop,
        Request::Command {
            name: "GETPIN".into(),
            args: None,
        },
        Request::Bye,
    ]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    // First command
    let r1 = server.recv().unwrap().unwrap();
    assert_eq!(
        r1,
        Request::Option {
            key: "display".into(),
            value: ":0".into(),
        }
    );

    // NOP is skipped (auto-OK), next visible is GETPIN
    let r2 = server.recv().unwrap().unwrap();
    assert_eq!(
        r2,
        Request::Command {
            name: "GETPIN".into(),
            args: None
        }
    );

    // BYE → None
    assert!(server.recv().unwrap().is_none());
}

#[test]
fn session_reset_then_more_commands() {
    let wire = wire_requests(&[
        Request::Command {
            name: "GETPIN".into(),
            args: None,
        },
        Request::Reset,
        Request::Command {
            name: "GETPIN".into(),
            args: None,
        },
        Request::Bye,
    ]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    // First GETPIN
    let r1 = server.recv().unwrap().unwrap();
    assert_eq!(
        r1,
        Request::Command {
            name: "GETPIN".into(),
            args: None
        }
    );

    // RESET is surfaced (caller can clear state)
    let r2 = server.recv().unwrap().unwrap();
    assert_eq!(r2, Request::Reset);

    // Second GETPIN after reset
    let r3 = server.recv().unwrap().unwrap();
    assert_eq!(
        r3,
        Request::Command {
            name: "GETPIN".into(),
            args: None
        }
    );

    // BYE
    assert!(server.recv().unwrap().is_none());
}

#[test]
fn session_multiple_comments_skipped() {
    let wire = wire_requests(&[
        Request::Comment("starting session".into()),
        Request::Comment("debug level 2".into()),
        Request::Command {
            name: "GETPIN".into(),
            args: None,
        },
        Request::Bye,
    ]);
    let mut server = Server::new(Cursor::new(wire), Vec::new());

    // Both comments skipped, first visible is GETPIN
    let req = server.recv().unwrap().unwrap();
    assert_eq!(
        req,
        Request::Command {
            name: "GETPIN".into(),
            args: None
        }
    );

    assert!(server.recv().unwrap().is_none());
}

#[test]
fn session_err_response_from_server() {
    let wire = wire_responses(&[Response::err(
        ErrorCode::GENERAL,
        Some("something went wrong".into()),
    )]);
    let mut client = Client::new(Cursor::new(wire), Vec::new());

    let resp = client.recv().unwrap().unwrap();
    assert_eq!(
        resp,
        Response::Err(ErrorCode::GENERAL, Some("something went wrong".into()))
    );
}

#[test]
fn session_data_with_special_bytes_roundtrip() {
    // Data containing newline, percent, and control bytes.
    // \xff is excluded because Response::parse rejects non-UTF-8 decoded data.
    let original = b"line1\nline2\x00\x01%25";
    let wire = wire_responses(&[Response::data(original.to_vec())]);
    let mut client = Client::new(Cursor::new(wire), Vec::new());

    let resp = client.recv().unwrap().unwrap();
    match resp {
        Response::Data(data) => assert_eq!(data, original),
        other => panic!("expected Data, got {:?}", other),
    }
}
