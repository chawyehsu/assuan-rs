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
        match req {
            Request::Bye => self.line_writer.write(b"BYE"),
            Request::Reset => self.line_writer.write(b"RESET"),
            Request::Nop => self.line_writer.write(b"NOP"),
            Request::End => self.line_writer.write(b"END"),
            Request::Cancel => self.line_writer.write(b"CAN"),
            Request::Help => self.line_writer.write(b"HELP"),
            Request::Quit => self.line_writer.write(b"QUIT"),
            Request::Auth => self.line_writer.write(b"AUTH"),
            Request::Option { key, value } => {
                if value.is_empty() {
                    let line = format!("OPTION {key}");
                    self.line_writer.write(line.as_bytes())
                } else {
                    let line = format!("OPTION {key}={value}");
                    self.line_writer.write(line.as_bytes())
                }
            }
            Request::Data(data) => self.line_writer.write_data(&data),
            Request::Comment(s) => {
                let line = format!("# {s}");
                self.line_writer.write(line.as_bytes())
            }
            Request::Command { name, args } => {
                match args {
                    None => self.line_writer.write(name.as_bytes()),
                    Some(args) => {
                        // Percent-encode the args.
                        let encoded_len = crate::percent::encoded_len(args.len());
                        let mut buf = Vec::with_capacity(name.len() + 1 + encoded_len);
                        buf.extend_from_slice(name.as_bytes());
                        buf.push(b' ');
                        let mut enc_buf = vec![0u8; encoded_len];
                        let n = crate::percent::encode(args.as_bytes(), &mut enc_buf);
                        buf.extend_from_slice(&enc_buf[..n]);
                        self.line_writer.write(&buf)
                    }
                }
            }
        }
    }

    /// Receive the next response from the server.
    ///
    /// Returns `Ok(None)` on clean EOF.
    pub fn recv(&mut self) -> Result<Option<Response>, Error> {
        let line = match self.line_reader.read(&mut self.reader) {
            Ok(Some(line)) => line,
            Ok(None) => return Ok(None),
            Err(e) => return Err(e),
        };

        let resp = Response::parse(line)?;
        Ok(Some(resp))
    }
}
