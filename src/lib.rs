//! Assuan protocol primitives in Rust.
//!
//! This crate provides reusable building blocks for implementing
//! [Assuan protocol](https://www.gnupg.org/documentation/manuals/assuan/index.html)
//! communication, primarily used in the GPG ecosystem.
//!
//! It focuses on protocol-level primitives only:
//! - **Reading**: [`LineReader`] — buffered line reader with spec-enforced limits
//! - **Parsing**: [`Request`] — parse a line into command + arguments
//! - **Responses**: [`Response`] — typed response lines with owned data
//! - **Server**: [`Server`] — concrete server with `send`/`recv`
//! - **Client**: [`Client`] — concrete client with `send`/`recv`
//! - **Errors**: [`ErrorCode`] — GPG/Assuan error codes

#![forbid(unused_crate_dependencies)]
#![deny(missing_docs)]

mod client;
mod error;
mod line_reader;
mod line_writer;
mod percent;
mod request;
mod response;
mod server;

pub use client::Client;
pub use error::{Error, ErrorCode};
pub use line_reader::LineReader;
pub use request::Request;
pub use response::Response;
pub use server::Server;

/// Maximum size of an Assuan line in bytes, as specified by the protocol.
pub const MAX_LINE_SIZE: usize = 1000;
