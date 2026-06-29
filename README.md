# assuan

> Assuan protocol primitives in Rust

[![crates-svg]][crates-url]
[![license][license-badge]](LICENSE-APACHE)
[![codecov][codecov-badge]][codecov]

A library of reusable building blocks for implementing
[Assuan protocol](https://www.gnupg.org/documentation/manuals/assuan/index.html)
communication, primarily used in the GPG ecosystem.

## Usage

Add `assuan` to your `Cargo.toml`:

```sh
cargo add assuan
```

### Parse a Request

```rust
use assuan::Request;

let mut line = b"SETDESC Enter passphrase".to_vec();
let req = Request::parse(&mut line).unwrap();
assert_eq!(req, Request::Command {
    name: "SETDESC".into(),
    args: Some("Enter passphrase".into()),
});
```

### Build a Response

```rust
use assuan::{Response, ErrorCode};

// OK with no message (zero-alloc, const)
let ok = Response::OK;

// OK with a message
let ok = Response::ok("protocol version 1");

// Error with code and message
let err = Response::err(ErrorCode::GENERAL, Some("oops".into()));

// Status, data, inquire
let status = Response::status("BUTTON_INFO", "close");
let data = Response::data(b"hello\nworld".to_vec());
let inq = Response::inquire("PASSPHRASE", "");
```

### Use the Server and Client

`Server` and `Client` wrap any `Read + Write` transport, providing typed
`send`/`recv`. The server transparently handles protocol-level commands
(`BYE`, `NOP`, comments).

```rust
use assuan::{Server, Client, Request, Response};

// Server side — e.g. over stdin/stdout
let mut server = Server::new(stdin, stdout);
while let Some(req) = server.recv()? {
    match req {
        Request::Command { name, args } => {
            // handle application commands...
            server.send(Response::OK)?;
        }
        _ => {}
    }
}

// Client side
let mut client = Client::new(reader, writer);
client.send(Request::Command { name: "GETPIN".into(), args: None })?;
if let Some(resp) = client.recv()? {
    // handle response...
}
```

## API

| Type | Description |
| ------ | ------------- |
| [`Request`] | Client request — protocol commands, data lines, and application commands |
| [`Response`] | Server response — OK, ERR, S, D, INQUIRE, comments |
| [`Server`] | Concrete server with typed `send`/`recv`, handles BYE/NOP transparently |
| [`Client`] | Concrete client with typed `send`/`recv` |
| [`LineReader`] | Buffered line reader with 1000-byte line limit, zero-alloc hot path |
| [`Error`] | Crate error type — I/O and protocol-level errors |
| [`ErrorCode`] | `u32` newtype for libgpg-error compatible error codes |

## License

**assuan** © [Chawye Hsu](https://github.com/chawyehsu). Released under the
[MIT](LICENSE-MIT) / [Apache-2.0](LICENSE-APACHE) license.

> [Blog](https://chawyehsu.com) · GitHub [@chawyehsu](https://github.com/chawyehsu) · Twitter [@chawyehsu](https://twitter.com/chawyehsu)

[crates-svg]: https://img.shields.io/crates/v/assuan.svg?style=flat&logo=rust
[crates-url]: https://crates.io/crates/assuan
[license-badge]: https://img.shields.io/github/license/chawyehsu/assuan-rs?style=flat&logo=spdx
[codecov-badge]: https://img.shields.io/codecov/c/gh/chawyehsu/assuan-rs?style=flat&logo=codecov
[codecov]: https://codecov.io/github/chawyehsu/assuan-rs
