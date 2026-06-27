//! Internal line writer for the Assuan protocol.
//!
//! Writes complete lines to an [`io::Write`] sink, enforcing the
//! [`MAX_LINE_SIZE`] limit and appending a trailing `\n`.

use std::io::Write;

use crate::MAX_LINE_SIZE;
use crate::error::Error;
use crate::percent;

/// Buffered line writer with a fixed internal buffer.
///
/// Writes complete Assuan protocol lines to an underlying [`io::Write`].
/// Enforces the 1000-byte line length limit.
pub(crate) struct LineWriter<W: Write> {
    writer: W,
}

impl<W: Write> LineWriter<W> {
    /// Create a new line writer wrapping the given writer.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write a raw line, appending `\n`.
    ///
    /// The line must not exceed [`MAX_LINE_SIZE`] bytes (including the `\n`).
    /// Returns the number of bytes written.
    pub fn write_line(&mut self, line: &[u8]) -> Result<usize, Error> {
        let total = line.len() + 1; // +1 for \n
        if total > MAX_LINE_SIZE {
            return Err(Error::LineTooLong);
        }
        self.writer.write_all(line).map_err(Error::Io)?;
        self.writer.write_all(b"\n").map_err(Error::Io)?;
        Ok(total)
    }

    /// Write a data line (`D <percent-encoded-data>\n`).
    ///
    /// The raw `data` is percent-encoded before writing. Returns the number
    /// of bytes written.
    pub fn write_data_line(&mut self, data: &[u8]) -> Result<usize, Error> {
        let max_encoded = MAX_LINE_SIZE - 3; // "D " (2) + "\n" (1)
        let encoded_len = percent::encoded_len(data.len());
        if encoded_len > max_encoded {
            return Err(Error::LineTooLong);
        }

        let mut buf = [0u8; MAX_LINE_SIZE];
        buf[0] = b'D';
        buf[1] = b' ';
        let n = percent::encode(data, &mut buf[2..]);
        buf[2 + n] = b'\n';
        let total = 2 + n + 1;

        self.writer.write_all(&buf[..total]).map_err(Error::Io)?;
        Ok(total)
    }

    /// Access the underlying writer.
    pub fn inner(&mut self) -> &mut W {
        &mut self.writer
    }
}
