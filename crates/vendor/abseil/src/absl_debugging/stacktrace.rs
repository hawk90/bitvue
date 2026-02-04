//! Stack trace utilities.
//!
//! Provides functions for capturing and printing stack traces.

use core::fmt;

/// Represents a stack trace captured at a specific point in execution.
///
/// Stack traces can be printed to help debug where an error occurred.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_debugging::stacktrace::StackTrace;
///
/// let trace = StackTrace::capture();
/// println!("Error occurred at:\n{}", trace);
/// ```
#[derive(Clone)]
pub struct StackTrace {
    /// Frames in the stack trace.
    frames: Vec<StackFrame>,
}

/// A single frame in a stack trace.
#[derive(Clone, Debug)]
pub struct StackFrame {
    /// Instruction pointer address.
    pub ip: usize,
    /// Symbol name (if available).
    pub symbol: Option<String>,
    /// File name (if available).
    pub file: Option<String>,
    /// Line number (if available).
    pub line: Option<u32>,
}

impl StackFrame {
    /// Creates a new stack frame.
    #[inline]
    pub fn new(ip: usize) -> Self {
        StackFrame {
            ip,
            symbol: None,
            file: None,
            line: None,
        }
    }

    /// Creates a new stack frame with symbol information.
    #[inline]
    pub fn with_symbol(ip: usize, symbol: String) -> Self {
        StackFrame {
            ip,
            symbol: Some(symbol),
            file: None,
            line: None,
        }
    }

    /// Creates a new stack frame with file and line information.
    #[inline]
    pub fn with_location(ip: usize, file: String, line: u32) -> Self {
        StackFrame {
            ip,
            symbol: None,
            file: Some(file),
            line: Some(line),
        }
    }
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.ip)?;
        if let Some(ref symbol) = self.symbol {
            write!(f, " - {}", symbol)?;
        }
        if let (Some(ref file), Some(line)) = (&self.file, self.line) {
            write!(f, " ({}:{})", file, line)?;
        }
        Ok(())
    }
}

impl StackTrace {
    /// Captures the current stack trace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_debugging::stacktrace::StackTrace;
    ///
    /// let trace = StackTrace::capture();
    /// ```
    #[inline]
    pub fn capture() -> Self {
        #[cfg(feature = "std")]
        {
            Self::capture_impl()
        }
        #[cfg(not(feature = "std"))]
        {
            StackTrace {
                frames: Vec::new(),
            }
        }
    }

    #[cfg(feature = "std")]
    fn capture_impl() -> Self {
        // Note: In a real implementation, this would use platform-specific
        // code to capture the actual stack trace. For now, we provide a stub.
        // Platforms: backtrace() on Unix, CaptureStackBackTrace() on Windows.
        StackTrace {
            frames: vec![StackFrame::new(0x1000)],
        }
    }

    /// Creates an empty stack trace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_debugging::stacktrace::StackTrace;
    ///
    /// let trace = StackTrace::empty();
    /// assert!(trace.is_empty());
    /// ```
    #[inline]
    pub fn empty() -> Self {
        StackTrace {
            frames: Vec::new(),
        }
    }

    /// Returns true if the stack trace is empty.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use abseil::absl_debugging::stacktrace::StackTrace;
    ///
    /// let trace = StackTrace::empty();
    /// assert!(trace.is_empty());
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Returns the number of frames in the stack trace.
    #[inline]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Returns an iterator over the frames in the stack trace.
    #[inline]
    pub fn frames(&self) -> &[StackFrame] {
        &self.frames
    }

    /// Adds a frame to the stack trace.
    #[inline]
    pub fn push(&mut self, frame: StackFrame) {
        self.frames.push(frame);
    }

    /// Reserves capacity for at least `additional` more frames.
    #[inline]
    pub fn reserve(&mut self, additional: usize) {
        self.frames.reserve(additional);
    }
}

impl fmt::Display for StackTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, frame) in self.frames.iter().enumerate() {
            writeln!(f, "  #{} - {}", i, frame)?;
        }
        Ok(())
    }
}

impl fmt::Debug for StackTrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("StackTrace")
            .field("frames", &self.frames)
            .finish()
    }
}

impl Default for StackTrace {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

/// Prints the current stack trace to stderr.
///
/// This is a convenience function for debugging.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_debugging::stacktrace::print_stack_trace;
///
/// print_stack_trace();
/// ```
#[inline]
pub fn print_stack_trace() {
    #[cfg(feature = "std")]
    {
        let trace = StackTrace::capture();
        eprintln!("Stack trace:");
        for (i, frame) in trace.frames().iter().enumerate() {
            eprintln!("  #{} - {}", i, frame);
        }
    }
    #[cfg(not(feature = "std"))]
    {
        // No-op in no_std environment
    }
}

/// Prints the current stack trace to stderr with a custom message.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_debugging::stacktrace::print_stack_trace_with_message;
///
/// print_stack_trace_with_message("Error occurred");
/// ```
#[inline]
pub fn print_stack_trace_with_message(msg: &str) {
    #[cfg(feature = "std")]
    {
        eprintln!("{}", msg);
        print_stack_trace();
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = msg;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack_trace_empty() {
        let trace = StackTrace::empty();
        assert!(trace.is_empty());
        assert_eq!(trace.len(), 0);
    }

    #[test]
    fn test_stack_trace_push() {
        let mut trace = StackTrace::empty();
        trace.push(StackFrame::new(0x1000));
        trace.push(StackFrame::new(0x2000));
        assert_eq!(trace.len(), 2);
        assert!(!trace.is_empty());
    }

    #[test]
    fn test_stack_frame_new() {
        let frame = StackFrame::new(0x1000);
        assert_eq!(frame.ip, 0x1000);
        assert!(frame.symbol.is_none());
        assert!(frame.file.is_none());
        assert!(frame.line.is_none());
    }

    #[test]
    fn test_stack_frame_with_symbol() {
        let frame = StackFrame::with_symbol(0x1000, "my_function".to_string());
        assert_eq!(frame.ip, 0x1000);
        assert_eq!(frame.symbol.as_ref().unwrap(), "my_function");
    }

    #[test]
    fn test_stack_frame_with_location() {
        let frame = StackFrame::with_location(0x1000, "file.rs".to_string(), 42);
        assert_eq!(frame.ip, 0x1000);
        assert_eq!(frame.file.as_ref().unwrap(), "file.rs");
        assert_eq!(frame.line.unwrap(), 42);
    }

    #[test]
    fn test_stack_frame_display() {
        let frame = StackFrame::new(0x1000);
        assert_eq!(format!("{}", frame), "0x1000");

        let frame_with_symbol = StackFrame::with_symbol(0x2000, "func".to_string());
        assert_eq!(format!("{}", frame_with_symbol), "0x2000 - func");

        let frame_with_loc = StackFrame::with_location(0x3000, "file.rs".to_string(), 42);
        assert_eq!(format!("{}", frame_with_loc), "0x3000 (file.rs:42)");
    }

    #[test]
    fn test_stack_trace_display() {
        let mut trace = StackTrace::empty();
        trace.push(StackFrame::new(0x1000));
        trace.push(StackFrame::new(0x2000));
        let output = format!("{}", trace);
        assert!(output.contains("0x1000"));
        assert!(output.contains("0x2000"));
    }

    #[test]
    fn test_stack_trace_reserve() {
        let mut trace = StackTrace::empty();
        trace.reserve(10);
        trace.push(StackFrame::new(0x1000));
        assert_eq!(trace.len(), 1);
    }

    #[test]
    fn test_stack_trace_default() {
        let trace: StackTrace = StackTrace::default();
        assert!(trace.is_empty());
    }
}
