//! Stack trace options and formatting.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use super::backtrace::Backtrace;

/// Stack trace filtering options.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StackTraceOptions {
    /// Maximum number of frames to capture.
    pub max_frames: usize,
    /// Skip frames matching these patterns.
    pub skip_patterns: Vec<String>,
    /// Only include frames matching these patterns.
    pub filter_patterns: Vec<String>,
    /// Whether to resolve symbols.
    pub resolve_symbols: bool,
    /// Whether to include file/line information.
    pub include_location: bool,
}

impl StackTraceOptions {
    /// Creates a new default options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of frames.
    pub fn with_max_frames(mut self, max_frames: usize) -> Self {
        self.max_frames = max_frames;
        self
    }

    /// Adds a skip pattern.
    pub fn with_skip_pattern(mut self, pattern: String) -> Self {
        self.skip_patterns.push(pattern);
        self
    }

    /// Adds a filter pattern.
    pub fn with_filter_pattern(mut self, pattern: String) -> Self {
        self.filter_patterns.push(pattern);
        self
    }

    /// Enables symbol resolution.
    pub fn with_symbols(mut self, resolve: bool) -> Self {
        self.resolve_symbols = resolve;
        self
    }

    /// Enables location information.
    pub fn with_location(mut self, include: bool) -> Self {
        self.include_location = include;
        self
    }
}

/// Stack trace formatter.
#[derive(Clone, Debug)]
pub struct StackTraceFormatter {
    /// Show frame indices.
    pub show_indices: bool,
    /// Show addresses.
    pub show_addresses: bool,
    /// Show symbols.
    pub show_symbols: bool,
    /// Show file/line.
    pub show_location: bool,
    /// Indentation string.
    pub indent: String,
    /// Line separator.
    pub separator: String,
}

impl StackTraceFormatter {
    /// Creates a new default formatter.
    pub fn new() -> Self {
        Self {
            show_indices: true,
            show_addresses: true,
            show_symbols: true,
            show_location: true,
            indent: "  ".to_string(),
            separator: "\n".to_string(),
        }
    }

    /// Creates a compact formatter (addresses only).
    pub fn compact() -> Self {
        Self {
            show_indices: false,
            show_addresses: true,
            show_symbols: false,
            show_location: false,
            indent: "".to_string(),
            separator: " ".to_string(),
        }
    }

    /// Creates a verbose formatter (all information).
    pub fn verbose() -> Self {
        Self {
            show_indices: true,
            show_addresses: true,
            show_symbols: true,
            show_location: true,
            indent: "  ".to_string(),
            separator: "\n".to_string(),
        }
    }

    /// Formats a backtrace to a string.
    pub fn format(&self, backtrace: &Backtrace) -> String {
        let mut result = String::new();
        result.push_str("stack backtrace:");
        result.push_str(&self.separator);

        for (i, frame) in backtrace.frames().iter().enumerate() {
            result.push_str(&self.indent);
            if self.show_indices {
                result.push_str(&format!("{}: ", i));
            }
            if self.show_addresses {
                result.push_str(&format!("{:#x}", frame.ip));
            }
            if self.show_symbols {
                if let Some(ref symbol) = frame.symbol {
                    result.push_str(&format!(" - {}", symbol));
                }
            }
            if self.show_location {
                if let Some(ref file) = frame.file {
                    result.push_str(&format!(" ({}:{}", file, frame.line.unwrap_or(0)));
                }
            }
            result.push_str(&self.separator);
        }
        result
    }
}

impl Default for StackTraceFormatter {
    fn default() -> Self {
        Self::new()
    }
}

/// Stack trace analysis utilities.
pub struct StackTraceAnalysis;

impl StackTraceAnalysis {
    /// Finds duplicate frames in a backtrace.
    pub fn find_duplicates(backtrace: &Backtrace) -> Vec<(usize, usize)> {
        let mut duplicates = Vec::new();
        let frames = backtrace.frames();

        for i in 0..frames.len() {
            for j in (i + 1)..frames.len() {
                if frames[i].ip == frames[j].ip {
                    duplicates.push((i, j));
                }
            }
        }
        duplicates
    }

    /// Counts occurrences of each symbol.
    pub fn count_symbols(backtrace: &Backtrace) -> Vec<(&str, usize)> {
        use alloc::collections::BTreeMap;

        let mut counts = BTreeMap::new();

        for frame in backtrace.frames() {
            if let Some(ref symbol) = frame.symbol {
                *counts.entry(symbol.as_str()).or_insert(0) += 1;
            }
        }

        let mut result: Vec<_> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }

    /// Finds recursive calls (same symbol appearing multiple times).
    pub fn find_recursion(backtrace: &Backtrace) -> Vec<&str> {
        use alloc::collections::BTreeSet;

        let mut seen = BTreeSet::new();
        let mut recursive = Vec::new();

        for frame in backtrace.frames() {
            if let Some(ref symbol) = frame.symbol {
                if !seen.insert(symbol.as_str()) {
                    recursive.push(symbol.as_str());
                }
            }
        }
        recursive
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::backtrace::StackFrame;

    // Tests for StackTraceOptions
    #[test]
    fn test_stack_trace_options_new() {
        let options = StackTraceOptions::new();
        assert_eq!(options.max_frames, 0);
        assert!(options.skip_patterns.is_empty());
        assert!(options.filter_patterns.is_empty());
        assert!(options.resolve_symbols);
        assert!(options.include_location);
    }

    #[test]
    fn test_stack_trace_options_with_max_frames() {
        let options = StackTraceOptions::new().with_max_frames(10);
        assert_eq!(options.max_frames, 10);
    }

    #[test]
    fn test_stack_trace_options_with_skip_pattern() {
        let options = StackTraceOptions::new()
            .with_skip_pattern("test".to_string());
        assert_eq!(options.skip_patterns.len(), 1);
    }

    #[test]
    fn test_stack_trace_options_with_filter_pattern() {
        let options = StackTraceOptions::new()
            .with_filter_pattern("my_module".to_string());
        assert_eq!(options.filter_patterns.len(), 1);
    }

    #[test]
    fn test_stack_trace_options_with_symbols() {
        let options = StackTraceOptions::new().with_symbols(false);
        assert!(!options.resolve_symbols);
    }

    #[test]
    fn test_stack_trace_options_with_location() {
        let options = StackTraceOptions::new().with_location(false);
        assert!(!options.include_location);
    }

    // Tests for StackTraceFormatter
    #[test]
    fn test_stack_trace_formatter_new() {
        let formatter = StackTraceFormatter::new();
        assert!(formatter.show_indices);
        assert!(formatter.show_addresses);
        assert!(formatter.show_symbols);
        assert!(formatter.show_location);
    }

    #[test]
    fn test_stack_trace_formatter_compact() {
        let formatter = StackTraceFormatter::compact();
        assert!(!formatter.show_indices);
        assert!(formatter.show_addresses);
        assert!(!formatter.show_symbols);
        assert!(!formatter.show_location);
    }

    #[test]
    fn test_stack_trace_formatter_verbose() {
        let formatter = StackTraceFormatter::verbose();
        assert!(formatter.show_indices);
        assert!(formatter.show_addresses);
        assert!(formatter.show_symbols);
        assert!(formatter.show_location);
    }

    #[test]
    fn test_stack_trace_formatter_format() {
        let formatter = StackTraceFormatter::new();
        let frames = vec![
            StackFrame::new(0x1000).with_symbol("func1".to_string()),
        ];
        let bt = Backtrace::from_frames(frames);
        let formatted = formatter.format(&bt);
        assert!(formatted.contains("stack backtrace"));
    }

    #[test]
    fn test_stack_trace_formatter_default() {
        let formatter = StackTraceFormatter::default();
        assert!(formatter.show_indices);
    }

    // Tests for StackTraceAnalysis
    #[test]
    fn test_stack_trace_analysis_find_duplicates() {
        let frames = vec![
            StackFrame::new(0x1000),
            StackFrame::new(0x2000),
            StackFrame::new(0x1000), // Duplicate
        ];
        let bt = Backtrace::from_frames(frames);
        let duplicates = StackTraceAnalysis::find_duplicates(&bt);
        assert_eq!(duplicates.len(), 1);
        assert_eq!(duplicates[0], (0, 2));
    }

    #[test]
    fn test_stack_trace_analysis_count_symbols() {
        let frames = vec![
            StackFrame::new(0x1000).with_symbol("func_a".to_string()),
            StackFrame::new(0x2000).with_symbol("func_b".to_string()),
            StackFrame::new(0x3000).with_symbol("func_a".to_string()),
        ];
        let bt = Backtrace::from_frames(frames);
        let counts = StackTraceAnalysis::count_symbols(&bt);
        assert_eq!(counts[0].1, 2); // func_a appears twice
    }

    #[test]
    fn test_stack_trace_analysis_find_recursion() {
        let frames = vec![
            StackFrame::new(0x1000).with_symbol("recursive_func".to_string()),
            StackFrame::new(0x2000).with_symbol("recursive_func".to_string()),
            StackFrame::new(0x3000).with_symbol("other_func".to_string()),
        ];
        let bt = Backtrace::from_frames(frames);
        let recursion = StackTraceAnalysis::find_recursion(&bt);
        assert_eq!(recursion.len(), 1);
        assert_eq!(recursion[0], "recursive_func");
    }
}
