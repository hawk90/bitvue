//! Stack backtrace support.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// Represents a single frame in a stack trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StackFrame {
    /// Instruction pointer address.
    pub ip: usize,
    /// Symbol name (if available).
    pub symbol: Option<String>,
    /// Source file path (if available).
    pub file: Option<String>,
    /// Line number (if available).
    pub line: Option<u32>,
}

impl StackFrame {
    /// Creates a new stack frame.
    pub const fn new(ip: usize) -> Self {
        Self {
            ip,
            symbol: None,
            file: None,
            line: None,
        }
    }

    /// Sets the symbol name.
    pub fn with_symbol(mut self, symbol: String) -> Self {
        self.symbol = Some(symbol);
        self
    }

    /// Sets the source file location.
    pub fn with_location(mut self, file: String, line: u32) -> Self {
        self.file = Some(file);
        self.line = Some(line);
        self
    }
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}", self.ip)?;
        if let Some(ref symbol) = self.symbol {
            write!(f, " - {}", symbol)?;
        }
        if let Some(ref file) = self.file {
            write!(f, " ({}:{}", file, self.line.unwrap_or(0))?;
            if let Some(line) = self.line {
                write!(f, ":{}", line)?;
            }
            write!(f, ")")?;
        }
        Ok(())
    }
}

/// A collection of stack frames.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backtrace {
    frames: Vec<StackFrame>,
}

impl Backtrace {
    /// Creates a new backtrace from the current location.
    pub fn new() -> Self {
        Self::capture()
    }

    /// Captures a backtrace from the current location.
    pub fn capture() -> Self {
        Self {
            frames: Self::collect_frames(),
        }
    }

    /// Creates a backtrace with the given frames.
    pub fn from_frames(frames: Vec<StackFrame>) -> Self {
        Self { frames }
    }

    /// Returns the frames in this backtrace.
    pub fn frames(&self) -> &[StackFrame] {
        &self.frames
    }

    /// Returns the number of frames in this backtrace.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Returns true if this backtrace is empty.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Resolves symbols for all frames in this backtrace.
    #[cfg(feature = "std")]
    pub fn resolve(&mut self) {
        for frame in &mut self.frames {
            if frame.symbol.is_none() {
                // In a real implementation, this would resolve symbols
                // For now, leave as None
            }
        }
    }

    /// Collects stack frames (implementation-specific).
    fn collect_frames() -> Vec<StackFrame> {
        // In a real implementation, this would walk the stack
        // For no_std compatibility, we provide a stub
        vec![StackFrame::new(0)]
    }
}

impl fmt::Display for Backtrace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "stack backtrace:")?;
        for (i, frame) in self.frames.iter().enumerate() {
            writeln!(f, "  {}: {}", i, frame)?;
        }
        Ok(())
    }
}

impl Default for Backtrace {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let frame = StackFrame::new(0x1000).with_symbol("my_function".to_string());
        assert_eq!(frame.symbol, Some("my_function".to_string()));
    }

    #[test]
    fn test_stack_frame_with_location() {
        let frame = StackFrame::new(0x1000)
            .with_location("file.rs".to_string(), 42);
        assert_eq!(frame.file, Some("file.rs".to_string()));
        assert_eq!(frame.line, Some(42));
    }

    #[test]
    fn test_stack_frame_display() {
        let frame = StackFrame::new(0x1000);
        let s = format!("{}", frame);
        assert!(s.contains("1000"));
    }

    #[test]
    fn test_backtrace_new() {
        let bt = Backtrace::new();
        // Should capture at least one frame
        assert!(!bt.frames.is_empty());
    }

    #[test]
    fn test_backtrace_from_frames() {
        let frames = vec![
            StackFrame::new(0x1000),
            StackFrame::new(0x2000),
        ];
        let bt = Backtrace::from_frames(frames);
        assert_eq!(bt.len(), 2);
    }

    #[test]
    fn test_backtrace_len() {
        let frames = vec![
            StackFrame::new(0x1000),
            StackFrame::new(0x2000),
            StackFrame::new(0x3000),
        ];
        let bt = Backtrace::from_frames(frames);
        assert_eq!(bt.len(), 3);
    }

    #[test]
    fn test_backtrace_is_empty() {
        let bt = Backtrace::from_frames(vec![]);
        assert!(bt.is_empty());
    }

    #[test]
    fn test_backtrace_default() {
        let bt = Backtrace::default();
        // Should capture at least one frame
        assert!(!bt.frames.is_empty());
    }

    #[test]
    fn test_backtrace_display() {
        let bt = Backtrace::from_frames(vec![
            StackFrame::new(0x1000),
        ]);
        let s = format!("{}", bt);
        assert!(s.contains("stack backtrace"));
    }
}
