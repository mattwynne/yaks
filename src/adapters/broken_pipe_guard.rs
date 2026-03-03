// BrokenPipeGuard - wraps a Write implementation and silently absorbs BrokenPipe errors
//
// This is useful when writing to stdout that may be piped to a pager (like `less`).
// When the pager quits early, it closes the pipe, causing subsequent writes to fail
// with EPIPE (BrokenPipe). This adapter converts those errors to success, preventing
// panics from `.expect()` calls in display adapters.

use std::io::{self, ErrorKind, Write};

/// Wraps a `Write` implementation and converts `BrokenPipe` errors into success.
pub struct BrokenPipeGuard<W: Write> {
    inner: W,
}

impl<W: Write> BrokenPipeGuard<W> {
    /// Create a new BrokenPipeGuard wrapping the given writer.
    pub fn new(inner: W) -> Self {
        Self { inner }
    }
}

impl<W: Write> Write for BrokenPipeGuard<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.inner.write(buf) {
            Err(e) if e.kind() == ErrorKind::BrokenPipe => Ok(buf.len()),
            other => other,
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.inner.flush() {
            Err(e) if e.kind() == ErrorKind::BrokenPipe => Ok(()),
            other => other,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, ErrorKind};

    // Mock writer that always returns BrokenPipe errors
    struct BrokenPipeWriter;

    impl Write for BrokenPipeWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(Error::from(ErrorKind::BrokenPipe))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(Error::from(ErrorKind::BrokenPipe))
        }
    }

    // Mock writer that tracks calls
    struct MockWriter {
        writes: Vec<Vec<u8>>,
        flush_count: usize,
    }

    impl MockWriter {
        fn new() -> Self {
            Self {
                writes: Vec::new(),
                flush_count: 0,
            }
        }
    }

    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes.push(buf.to_vec());
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            self.flush_count += 1;
            Ok(())
        }
    }

    #[test]
    fn test_broken_pipe_write_returns_success() {
        let broken_writer = BrokenPipeWriter;
        let mut guard = BrokenPipeGuard::new(broken_writer);

        // Writing to a broken pipe should succeed (not return an error)
        let result = guard.write(b"test data");
        assert!(result.is_ok(), "write should succeed on BrokenPipe");
        assert_eq!(result.unwrap(), 9, "should return buffer length");
    }

    #[test]
    fn test_broken_pipe_flush_returns_success() {
        let broken_writer = BrokenPipeWriter;
        let mut guard = BrokenPipeGuard::new(broken_writer);

        // Flushing a broken pipe should succeed (not return an error)
        let result = guard.flush();
        assert!(result.is_ok(), "flush should succeed on BrokenPipe");
    }

    #[test]
    fn test_normal_writes_pass_through() {
        let mock_writer = MockWriter::new();
        let mut guard = BrokenPipeGuard::new(mock_writer);

        // Normal writes should pass through
        let result = guard.write(b"hello");
        assert!(result.is_ok(), "normal write should succeed");
        assert_eq!(result.unwrap(), 5, "should return actual bytes written");

        let result = guard.write(b"world");
        assert!(result.is_ok(), "second write should succeed");
        assert_eq!(result.unwrap(), 5, "should return actual bytes written");

        // Check that the inner writer received the data
        assert_eq!(guard.inner.writes.len(), 2, "should have 2 writes");
        assert_eq!(guard.inner.writes[0], b"hello");
        assert_eq!(guard.inner.writes[1], b"world");
    }

    #[test]
    fn test_normal_flush_passes_through() {
        let mock_writer = MockWriter::new();
        let mut guard = BrokenPipeGuard::new(mock_writer);

        let result = guard.flush();
        assert!(result.is_ok(), "normal flush should succeed");

        let result = guard.flush();
        assert!(result.is_ok(), "second flush should succeed");

        // Check that the inner writer received the flush calls
        assert_eq!(guard.inner.flush_count, 2, "should have 2 flushes");
    }

    #[test]
    fn test_other_errors_pass_through() {
        // Mock writer that returns a different error
        struct PermissionDeniedWriter;

        impl Write for PermissionDeniedWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(Error::from(ErrorKind::PermissionDenied))
            }

            fn flush(&mut self) -> io::Result<()> {
                Err(Error::from(ErrorKind::PermissionDenied))
            }
        }

        let error_writer = PermissionDeniedWriter;
        let mut guard = BrokenPipeGuard::new(error_writer);

        // Other errors should pass through unchanged
        let write_result = guard.write(b"test");
        assert!(write_result.is_err(), "other errors should pass through");
        assert_eq!(
            write_result.unwrap_err().kind(),
            ErrorKind::PermissionDenied,
            "should preserve error kind"
        );

        let flush_result = guard.flush();
        assert!(flush_result.is_err(), "other errors should pass through");
        assert_eq!(
            flush_result.unwrap_err().kind(),
            ErrorKind::PermissionDenied,
            "should preserve error kind"
        );
    }
}
