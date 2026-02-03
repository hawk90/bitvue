//! Debugging utilities.
//!
//! This module provides debugging utilities which help with debugging,
//! stack traces, and failure signals.
//!
//! # Examples
//!
//! ```rust
//! use abseil::absl_debugging::{get_stack_trace, StackTraceFormatter};
//!
//! // Print current stack trace
//! let trace = get_stack_trace();
//! println!("{}", trace);
//! ```

// Existing submodules
pub mod stacktrace;
pub mod symbolize;
pub mod failure_signal_handler;

// New organized submodules
mod backtrace;
mod failure;
mod symbol;
mod stack_trace;
mod breakpoint;
mod memory;
mod logging;

// Re-export from existing submodules
pub use stacktrace::{print_stack_trace, StackTrace};
pub use symbolize::{demangle, symbolize};

// Re-export backtrace types
pub use backtrace::{Backtrace, StackFrame};

// Re-export failure handling types
pub use failure::{
    register_failure_handler, ExtendedSignal, FailureContext, FailureHandler,
    FailureSignal, InstallFailureHandler, PrintFailureHandler, RegisterFailureHandler,
};

// Re-export symbol table types
pub use symbol::{Symbol, SymbolTable};

// Re-export stack trace utilities
pub use stack_trace::{StackTraceAnalysis, StackTraceFormatter, StackTraceOptions};

// Re-export breakpoint types
pub use breakpoint::{Breakpoint, BreakpointManager, Watchpoint, WatchpointType};

// Re-export memory types
pub use memory::{
    MemoryMap, MemoryPermissions, MemoryRegion, Profiler, ProfilingData,
};

// Re-export logging types
pub use logging::{
    AssertionOptions, AssertionError, DebugLogger, LogEntry, LogLevel,
};

// Re-export register state
pub use failure::RegisterState;

// Convenience functions

/// Demangles a symbol name.
///
/// This is a convenience function for `demangle::demangle_symbol`.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::demangle_symbol;
///
/// let mangled = "_ZN4core9panicking5panic17h50ba3113a19ff1a4E";
/// let demangled = demangle_symbol(mangled);
/// ```
pub fn demangle_symbol(symbol: &str) -> String {
    demangle(symbol)
}

/// Symbolizes an address to a symbol name.
///
/// This is a convenience function for `symbolize::symbolize_address`.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize_address;
///
/// let symbol = symbolize_address(0x1000);
/// ```
pub fn symbolize_address(address: usize) -> Option<String> {
    symbolize(address)
}

/// Returns the current stack trace as a string.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::get_stack_trace;
///
/// let trace = get_stack_trace();
/// ```
pub fn get_stack_trace() -> String {
    let backtrace = Backtrace::new();
    format!("{}", backtrace)
}

/// Prints the current stack trace.
///
/// This is a convenience function for `stacktrace::print_stack_trace()`.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::print_current_stack_trace;
///
/// print_current_stack_trace();
/// ```
pub fn print_current_stack_trace() {
    print_stack_trace();
}

/// Installs a global failure signal handler.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::{install_failure_handler, PrintFailureHandler};
///
/// install_failure_handler(&PrintFailureHandler);
/// ```
pub fn install_failure_handler(_handler: impl FailureHandler + 'static) {
    // Stub for no_std compatibility
}

/// Registers a custom failure handler.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::register_failure_handler;
///
/// register_failure_handler(Box::new(PrintFailureHandler));
/// ```
pub fn register_failure_handler(_handler: Box<dyn FailureHandler>) {
    // Stub for no_std compatibility
}

// Private traits for internal use
#[doc(hidden)]
pub trait InstallFailureHandler {}
#[doc(hidden)]
pub trait RegisterFailureHandler {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_stack_trace() {
        let trace = get_stack_trace();
        // Should contain some output
        assert!(!trace.is_empty());
    }

    #[test]
    fn test_demangle_symbol_function() {
        let symbol = "_ZN4core9panicking5panic17h50ba3113a19ff1a4E";
        let demangled = demangle_symbol(symbol);
        // The symbol should be demangled
        assert!(!demangled.is_empty());
    }

    #[test]
    fn test_symbolize_address_function() {
        let symbol = symbolize_address(0x1000);
        // Should return Option, might be None in no_std
        let _ = symbol;
    }
}
