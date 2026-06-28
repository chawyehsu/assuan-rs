//! Assuan server implementation.

use std::io::{Read, Write};

use crate::LineReader;
use crate::error::Error;
use crate::request::Request;
use crate::response::Response;

/// An Assuan protocol server.
///
/// Wraps a reader and writer, providing typed `send`/`recv` for Assuan
/// protocol communication. Handles protocol-level commands (BYE, NOP)
/// transparently.
pub struct Server<R: Read, W: Write> {
    reader: LineReader<R>,
    writer: W,
}

impl<R: Read, W: Write> Server<R, W> {
    /// Create a new server with the given reader and writer.
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: LineReader::new(reader),
            writer,
        }
    }

    /// Send a response to the client.
    ///
    /// Returns the number of bytes written.
    pub fn send(&mut self, resp: Response) -> Result<usize, Error> {
        resp.write_to(&mut self.writer)
    }

    /// Receive the next request from the client.
    ///
    /// Handles BYE and NOP transparently — they are never returned to the
    /// caller. BYE causes the next call to return `Ok(None)`. RESET is
    /// surfaced to the caller after sending OK. Comment lines are skipped.
    ///
    /// Returns `Ok(None)` on BYE or clean EOF.
    pub fn recv(&mut self) -> Result<Option<Request>, Error> {
        loop {
            let line = match self.reader.read() {
                Ok(Some(line)) => line,
                Ok(None) => return Ok(None),
                Err(e) => return Err(e),
            };

            let req = Request::parse(line)?;

            match &req {
                Request::Bye => {
                    // Send OK and return None to signal end of session.
                    Response::OK.write_to(&mut self.writer)?;
                    self.writer.flush().map_err(Error::Io)?;
                    return Ok(None);
                }
                Request::Nop => {
                    // Send OK, continue to next request.
                    Response::OK.write_to(&mut self.writer)?;
                    self.writer.flush().map_err(Error::Io)?;
                    continue;
                }
                Request::Comment(_) => {
                    // Comment lines are ignored per the Assuan spec.
                    continue;
                }
                Request::Reset => {
                    // Send OK, but surface to caller so they can clear state.
                    Response::OK.write_to(&mut self.writer)?;
                    self.writer.flush().map_err(Error::Io)?;
                    return Ok(Some(req));
                }
                _ => return Ok(Some(req)),
            }
        }
    }
}
