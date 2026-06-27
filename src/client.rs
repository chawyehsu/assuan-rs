//! Assuan client implementation.

use std::io::{Read, Write};

use crate::LineReader;
use crate::error::Error;
use crate::line_writer::LineWriter;
use crate::request::Request;
use crate::response::Response;

/// An Assuan protocol client.
///
/// Wraps a reader and writer, providing typed `send`/`recv` for Assuan
/// protocol communication.
pub struct Client<R: Read, W: Write> {
    reader: R,
    line_reader: LineReader,
    line_writer: LineWriter<W>,
}

impl<R: Read, W: Write> Client<R, W> {
    /// Create a new client with the given reader and writer.
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            line_reader: LineReader::new(),
            line_writer: LineWriter::new(writer),
        }
    }

    /// Send a request to the server.
    ///
    /// Returns the number of bytes written.
    pub fn send(&mut self, req: Request) -> Result<usize, Error> {
        req.write_to(self.line_writer.inner())
    }

    /// Receive the next response from the server.
    ///
    /// Returns `Ok(None)` on clean EOF.
    pub fn recv(&mut self) -> Result<Option<Response>, Error> {
        let line = match self.line_reader.read_line(&mut self.reader) {
            Ok(Some(line)) => line,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };

        let resp = Response::parse(line)?;
        Ok(Some(resp))
    }
}
