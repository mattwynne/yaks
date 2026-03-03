//! Animated terminal spinner for long-running operations.
//!
//! The spinner runs in a background thread, updating the terminal line
//! at a fixed interval until stopped.

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::domain::ports::ProgressHandle;

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const FRAME_INTERVAL: Duration = Duration::from_millis(80);

/// A handle to a running spinner. Dropping it stops the spinner.
pub struct SpinnerHandle {
    running: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl SpinnerHandle {
    /// Start a spinner with the given message, writing to the given writer.
    /// The spinner animates in a background thread until `stop()` is called
    /// or the handle is dropped.
    pub fn start(message: String, mut writer: Box<dyn Write + Send>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let thread = thread::spawn(move || {
            let mut frame_idx = 0;
            while running_clone.load(Ordering::Relaxed) {
                let frame = FRAMES[frame_idx % FRAMES.len()];
                let _ = write!(writer, "\r\x1b[2K\x1b[33m{frame}\x1b[0m {message}");
                let _ = writer.flush();
                frame_idx += 1;
                thread::sleep(FRAME_INTERVAL);
            }
            // Clear the spinner line on stop
            let _ = write!(writer, "\r\x1b[2K");
            let _ = writer.flush();
        });

        Self {
            running,
            thread: Some(thread),
        }
    }

    /// Stop the spinner and clear the line.
    pub fn stop(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl ProgressHandle for SpinnerHandle {}

impl Drop for SpinnerHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// A thread-safe buffer for capturing spinner output in tests.
    #[derive(Clone)]
    struct TestWriter(Arc<Mutex<Vec<u8>>>);

    impl TestWriter {
        fn new() -> Self {
            Self(Arc::new(Mutex::new(Vec::new())))
        }

        fn contents(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
        }
    }

    impl Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn spinner_writes_frames_to_output() {
        let buf = TestWriter::new();
        let handle = SpinnerHandle::start("Loading...".to_string(), Box::new(buf.clone()));

        // Let it spin for a few frames
        thread::sleep(Duration::from_millis(250));
        handle.stop();

        let output = buf.contents();
        // Should contain the message
        assert!(
            output.contains("Loading..."),
            "Expected 'Loading...' in output: {output:?}"
        );
        // Should contain spinner frame characters
        assert!(
            FRAMES.iter().any(|f| output.contains(f)),
            "Expected spinner frame in output: {output:?}"
        );
    }

    #[test]
    fn spinner_clears_line_on_stop() {
        let buf = TestWriter::new();
        let handle = SpinnerHandle::start("Working...".to_string(), Box::new(buf.clone()));

        thread::sleep(Duration::from_millis(100));
        handle.stop();

        let output = buf.contents();
        // Last thing written should be the clear sequence
        assert!(
            output.ends_with("\r\x1b[2K"),
            "Expected line clear at end of output: {output:?}"
        );
    }

    #[test]
    fn spinner_animates_multiple_frames() {
        let buf = TestWriter::new();
        let handle = SpinnerHandle::start("Spinning...".to_string(), Box::new(buf.clone()));

        // Wait long enough for multiple frames (80ms each)
        thread::sleep(Duration::from_millis(300));
        handle.stop();

        let output = buf.contents();
        // Count how many times the message appears (once per frame)
        let frame_count = output.matches("Spinning...").count();
        assert!(
            frame_count >= 3,
            "Expected at least 3 frames, got {frame_count}. Output: {output:?}"
        );
    }

    #[test]
    fn spinner_stops_on_drop() {
        let buf = TestWriter::new();
        {
            let _handle = SpinnerHandle::start("Dropping...".to_string(), Box::new(buf.clone()));
            thread::sleep(Duration::from_millis(100));
            // handle dropped here
        }

        let output = buf.contents();
        assert!(
            output.ends_with("\r\x1b[2K"),
            "Expected line clear on drop: {output:?}"
        );
    }
}
