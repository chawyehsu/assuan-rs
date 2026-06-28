//! Buffered line reader for the Assuan protocol.
//!
//! Reads lines from an [`io::Read`] source, enforcing the 1000-byte line length
//! limit specified by the Assuan specification. Uses a fixed internal buffer —
//! zero allocations on the hot path.

use std::io::{self, Read};

use crate::MAX_LINE_SIZE;
use crate::error::Error;

/// Buffered line reader with a fixed internal buffer.
///
/// Reads one line at a time from an [`io::Read`] source. Lines are delimited by
/// `\n`. The trailing `\n` is stripped from the returned slice. The returned
/// `&mut [u8]` borrows from the internal buffer and is valid until the next
/// call to [`read`](Self::read).
///
/// Returns a mutable slice so that callers (e.g., [`Request::parse`]) can
/// decode percent-encoded data in-place without allocation.
///
/// Lines longer than [`MAX_LINE_SIZE`] (1000 bytes) cause an
/// [`Error::LineTooLong`] return.
pub struct LineReader<R: Read> {
    /// The underlying reader.
    reader: R,

    /// Internal buffer for reading lines.
    buffer: [u8; MAX_LINE_SIZE],

    /// Number of bytes currently in the buffer.
    bytes_read: usize,

    /// Position of the last newline found in the buffer, if any.
    newline_found: Option<usize>,
}

impl<R: Read> LineReader<R> {
    /// Create a new line reader wrapping the given reader.
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: [0u8; MAX_LINE_SIZE],
            bytes_read: 0,
            newline_found: None,
        }
    }

    /// Read one line from the underlying reader.
    ///
    /// Returns `Ok(None)` on clean EOF (no partial data). Returns the line
    /// without trailing `\n` or `\r\n`. The returned slice is valid until the
    /// next call to `read`.
    ///
    /// Matches libassuan behavior: CRLF and LF line endings both produce the
    /// same clean output with no trailing CR.
    ///
    /// Returns [`Error::LineTooLong`] if a line exceeds [`MAX_LINE_SIZE`].
    pub fn read(&mut self) -> Result<Option<&mut [u8]>, Error> {
        // If the previous read found a newline, compact the buffer:
        // move leftover bytes to the front.
        if let Some(newline_pos) = self.newline_found.take() {
            let consumed = newline_pos + 1;
            self.bytes_read -= consumed;
            self.buffer.copy_within(consumed.., 0);
        }

        // Check if leftover bytes already contain a full line.
        if self.bytes_read != 0
            && let Some(pos) = self.buffer[..self.bytes_read]
                .iter()
                .position(|&b| b == b'\n')
        {
            self.newline_found = Some(pos);
            let end = strip_cr(&self.buffer[..pos]);
            return Ok(Some(&mut self.buffer[..end]));
        }

        // Read until we find a newline or fill the buffer.
        loop {
            if self.bytes_read >= MAX_LINE_SIZE {
                tracing::warn!(
                    "line too long ({} bytes), limit is {MAX_LINE_SIZE}",
                    self.bytes_read
                );
                return Err(Error::LineTooLong);
            }

            let n = match self.reader.read(&mut self.buffer[self.bytes_read..]) {
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(Error::Io(e)),
            };

            if n == 0 {
                // EOF.
                if self.bytes_read == 0 {
                    return Ok(None);
                }
                // Partial line at EOF — return it (some implementations do this).
                let end = self.bytes_read;
                self.bytes_read = 0;
                tracing::trace!(
                    "<< {} (partial EOF)",
                    String::from_utf8_lossy(&self.buffer[..end])
                );
                return Ok(Some(&mut self.buffer[..end]));
            }

            self.bytes_read += n;

            // Check for newline in the newly read bytes.
            let search_from = self.bytes_read - n;
            if let Some(pos) = self.buffer[search_from..self.bytes_read]
                .iter()
                .position(|&b| b == b'\n')
            {
                let abs_pos = search_from + pos;
                self.newline_found = Some(abs_pos);
                let end = strip_cr(&self.buffer[..abs_pos]);
                tracing::trace!("<< {}", String::from_utf8_lossy(&self.buffer[..end]));
                return Ok(Some(&mut self.buffer[..end]));
            }
        }
    }

}

/// Strip a trailing `\r` from a byte slice, returning the new length.
fn strip_cr(s: &[u8]) -> usize {
    if s.last() == Some(&b'\r') {
        s.len() - 1
    } else {
        s.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_nothing() {
        let mut reader = LineReader::new(io::Cursor::new(b"" as &[u8]));
        assert!(reader.read().unwrap().is_none());
    }

    #[test]
    fn reads_one_line() {
        let mut reader = LineReader::new(io::Cursor::new(b"a line\n" as &[u8]));
        let line = reader.read().unwrap().unwrap();
        assert_eq!(line, b"a line");
    }

    #[test]
    fn reads_two_lines() {
        let mut reader = LineReader::new(io::Cursor::new(b"line1\nline2\n" as &[u8]));

        let line1 = reader.read().unwrap().unwrap();
        assert_eq!(line1, b"line1");

        let line2 = reader.read().unwrap().unwrap();
        assert_eq!(line2, b"line2");
    }

    #[test]
    fn reads_line_then_eof() {
        let mut reader = LineReader::new(io::Cursor::new(b"a line\n" as &[u8]));

        let line = reader.read().unwrap().unwrap();
        assert_eq!(line, b"a line");

        let line2 = reader.read().unwrap();
        assert!(line2.is_none());
    }

    #[test]
    fn errors_on_line_too_long() {
        let long_line = vec![b'x'; MAX_LINE_SIZE + 1];
        let mut reader = LineReader::new(io::Cursor::new(long_line));
        assert!(matches!(
            reader.read(),
            Err(Error::LineTooLong)
        ));
    }

    #[test]
    fn reads_partial_line_at_eof() {
        let mut reader = LineReader::new(io::Cursor::new(b"partial" as &[u8]));
        let line = reader.read().unwrap().unwrap();
        assert_eq!(line, b"partial");
    }

    #[test]
    fn strips_crlf() {
        let mut reader = LineReader::new(io::Cursor::new(b"a line\r\n" as &[u8]));
        let line = reader.read().unwrap().unwrap();
        assert_eq!(line, b"a line");
    }

    #[test]
    fn strips_lf_only() {
        let mut reader = LineReader::new(io::Cursor::new(b"a line\n" as &[u8]));
        let line = reader.read().unwrap().unwrap();
        assert_eq!(line, b"a line");
    }

    #[test]
    fn crlf_two_lines() {
        let mut reader = LineReader::new(io::Cursor::new(b"line1\r\nline2\r\n" as &[u8]));

        let line1 = reader.read().unwrap().unwrap();
        assert_eq!(line1, b"line1");

        let line2 = reader.read().unwrap().unwrap();
        assert_eq!(line2, b"line2");
    }
}
