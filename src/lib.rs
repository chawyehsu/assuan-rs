//! Assuan protocol primitives in Rust.
//!
//! This crate provides reusable building blocks for implementing
//! [Assuan protocol](https://www.gnupg.org/documentation/manuals/assuan/index.html)
//! communication, primarily used in the GPG ecosystem.
//!
//! It focuses on protocol-level primitives only:
//! - **Reading**: [`LineReader`] — buffered line reader with spec-enforced limits
//! - **Parsing**: [`Request`] — parse a line into command + arguments
//! - **Writing**: [`write_data`], [`write_ok`], [`write_err`], [`write_status`]
//! - **Errors**: [`ErrorCode`] — GPG/Assuan error codes
//!
//! Command dispatch, server loops, and application logic are the consumer's
//! responsibility.

#![forbid(unused_crate_dependencies)]
#![deny(missing_docs)]

mod error;
mod line_reader;
mod percent;
mod request;
mod response;

pub use error::{Error, ErrorCode};
pub use line_reader::LineReader;
pub use request::Request;
pub use response::{write_data, write_err, write_ok, write_status};

/// Maximum size of an Assuan line in bytes, as specified by the protocol.
pub const MAX_LINE_SIZE: usize = 1000;
