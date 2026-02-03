//! Failure signal handling.

use alloc::string::String;
use core::fmt;

use super::backtrace::Backtrace;

/// Represents a failure signal (SIGSEGV, SIGABRT, etc.).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureSignal {
    /// Segmentation fault (invalid memory access).
    SigSegv,
    /// Abort (usually from assert failure).
    SigAbrt,
    /// Illegal instruction.
    SigIll,
    /// Floating point exception.
    SigFpe,
    /// Bus error (alignment fault).
    SigBus,
    /// Termination signal.
    SigTerm,
    /// Interrupt signal (Ctrl+C).
    SigInt,
    /// Unknown signal.
    Unknown(i32),
}

impl FailureSignal {
    /// Returns the name of this signal.
    pub fn name(&self) -> &'static str {
        match self {
            FailureSignal::SigSegv => "SIGSEGV",
            FailureSignal::SigAbrt => "SIGABRT",
            FailureSignal::SigIll => "SIGILL",
            FailureSignal::SigFpe => "SIGFPE",
            FailureSignal::SigBus => "SIGBUS",
            FailureSignal::SigTerm => "SIGTERM",
            FailureSignal::SigInt => "SIGINT",
            FailureSignal::Unknown(_) => "UNKNOWN",
        }
    }

    /// Returns a description of this signal.
    pub fn description(&self) -> &'static str {
        match self {
            FailureSignal::SigSegv => "Segmentation fault",
            FailureSignal::SigAbrt => "Abort",
            FailureSignal::SigIll => "Illegal instruction",
            FailureSignal::SigFpe => "Floating point exception",
            FailureSignal::SigBus => "Bus error",
            FailureSignal::SigTerm => "Termination",
            FailureSignal::SigInt => "Interrupt",
            FailureSignal::Unknown(_) => "Unknown signal",
        }
    }
}

impl fmt::Display for FailureSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name(), self.description())
    }
}

/// A handler for failure signals.
pub trait FailureHandler: Send + Sync {
    /// Called when a failure signal is received.
    fn handle_signal(&self, signal: FailureSignal, backtrace: &Backtrace);
}

/// A failure handler that prints to stderr.
#[derive(Clone, Debug, Default)]
pub struct PrintFailureHandler;

impl FailureHandler for PrintFailureHandler {
    fn handle_signal(&self, signal: FailureSignal, backtrace: &Backtrace) {
        eprintln!("Fatal error: {}", signal);
        eprintln!("{}", backtrace);
    }
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
    // In a real implementation, this would register signal handlers
    // For no_std compatibility, this is a stub
}

/// Registers a custom failure handler.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::{register_failure_handler, FailureHandler, FailureSignal, Backtrace};
///
/// struct MyHandler;
/// impl FailureHandler for MyHandler {
///     fn handle_signal(&self, signal: FailureSignal, backtrace: &Backtrace) {
///         // Custom handling
///     }
/// }
///
/// register_failure_handler(Box::new(MyHandler));
/// ```
pub fn register_failure_handler(_handler: Box<dyn FailureHandler>) {
    // In a real implementation, this would store the handler
    // For no_std compatibility, this is a stub
}

/// Additional failure signals beyond standard POSIX signals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExtendedSignal {
    /// Stack overflow.
    StackOverflow,
    /// Heap corruption detected.
    HeapCorruption,
    /// Double free detected.
    DoubleFree,
    /// Use after free detected.
    UseAfterFree,
    /// Buffer overflow detected.
    BufferOverflow,
    /// Null pointer dereference.
    NullDereference,
    /// Data race detected.
    DataRace,
    /// Deadlock detected.
    Deadlock,
    /// Timeout.
    Timeout,
    /// Out of memory.
    OutOfMemory,
}

impl ExtendedSignal {
    /// Returns the name of this signal.
    pub fn name(&self) -> &'static str {
        match self {
            ExtendedSignal::StackOverflow => "STACK_OVERFLOW",
            ExtendedSignal::HeapCorruption => "HEAP_CORRUPTION",
            ExtendedSignal::DoubleFree => "DOUBLE_FREE",
            ExtendedSignal::UseAfterFree => "USE_AFTER_FREE",
            ExtendedSignal::BufferOverflow => "BUFFER_OVERFLOW",
            ExtendedSignal::NullDereference => "NULL_DEREFERENCE",
            ExtendedSignal::DataRace => "DATA_RACE",
            ExtendedSignal::Deadlock => "DEADLOCK",
            ExtendedSignal::Timeout => "TIMEOUT",
            ExtendedSignal::OutOfMemory => "OUT_OF_MEMORY",
        }
    }

    /// Returns a description of this signal.
    pub fn description(&self) -> &'static str {
        match self {
            ExtendedSignal::StackOverflow => "Stack overflow detected",
            ExtendedSignal::HeapCorruption => "Heap corruption detected",
            ExtendedSignal::DoubleFree => "Double free detected",
            ExtendedSignal::UseAfterFree => "Use after free detected",
            ExtendedSignal::BufferOverflow => "Buffer overflow detected",
            ExtendedSignal::NullDereference => "Null pointer dereference",
            ExtendedSignal::DataRace => "Data race detected",
            ExtendedSignal::Deadlock => "Deadlock detected",
            ExtendedSignal::Timeout => "Operation timed out",
            ExtendedSignal::OutOfMemory => "Out of memory",
        }
    }
}

impl fmt::Display for ExtendedSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name(), self.description())
    }
}

/// A register state capture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RegisterState {
    /// Instruction pointer.
    pub ip: usize,
    /// Stack pointer.
    pub sp: usize,
    /// Frame pointer (if available).
    pub fp: Option<usize>,
    /// Additional registers (platform-specific).
    pub regs: [usize; 8],
}

impl RegisterState {
    /// Creates a new register state.
    pub const fn new() -> Self {
        Self {
            ip: 0,
            sp: 0,
            fp: None,
            regs: [0; 8],
        }
    }

    /// Sets the instruction pointer.
    pub const fn with_ip(mut self, ip: usize) -> Self {
        self.ip = ip;
        self
    }

    /// Sets the stack pointer.
    pub const fn with_sp(mut self, sp: usize) -> Self {
        self.sp = sp;
        self
    }

    /// Sets the frame pointer.
    pub const fn with_fp(mut self, fp: usize) -> Self {
        self.fp = Some(fp);
        self
    }
}

/// A failure context containing information about a failure.
#[derive(Clone, Debug)]
pub struct FailureContext {
    /// The signal that caused the failure.
    pub signal: FailureSignal,
    /// The backtrace at the time of failure.
    pub backtrace: Backtrace,
    /// The register state at the time of failure.
    pub registers: RegisterState,
    /// A description of the failure.
    pub description: String,
}

impl FailureContext {
    /// Creates a new failure context.
    pub fn new(signal: FailureSignal) -> Self {
        Self {
            signal,
            backtrace: Backtrace::new(),
            registers: RegisterState::new(),
            description: String::new(),
        }
    }

    /// Sets the backtrace.
    pub fn with_backtrace(mut self, backtrace: Backtrace) -> Self {
        self.backtrace = backtrace;
        self
    }

    /// Sets the register state.
    pub fn with_registers(mut self, registers: RegisterState) -> Self {
        self.registers = registers;
        self
    }

    /// Sets the description.
    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }
}

impl fmt::Display for FailureContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Failure: {}", self.signal)?;
        if !self.description.is_empty() {
            writeln!(f, "Description: {}", self.description)?;
        }
        writeln!(f, "Backtrace:")?;
        for (i, frame) in self.backtrace.frames().iter().enumerate() {
            writeln!(f, "  {}: {}", i, frame)?;
        }
        writeln!(f, "Registers:")?;
        writeln!(f, "  IP: {:#x}", self.registers.ip)?;
        writeln!(f, "  SP: {:#x}", self.registers.sp)?;
        if let Some(fp) = self.registers.fp {
            writeln!(f, "  FP: {:#x}", fp)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::backtrace::StackFrame;

    #[test]
    fn test_failure_signal_name() {
        assert_eq!(FailureSignal::SigSegv.name(), "SIGSEGV");
        assert_eq!(FailureSignal::SigAbrt.name(), "SIGABRT");
        assert_eq!(FailureSignal::SigIll.name(), "SIGILL");
        assert_eq!(FailureSignal::SigFpe.name(), "SIGFPE");
        assert_eq!(FailureSignal::SigBus.name(), "SIGBUS");
        assert_eq!(FailureSignal::SigTerm.name(), "SIGTERM");
        assert_eq!(FailureSignal::SigInt.name(), "SIGINT");
        assert_eq!(FailureSignal::Unknown(42).name(), "UNKNOWN");
    }

    #[test]
    fn test_failure_signal_description() {
        assert_eq!(FailureSignal::SigSegv.description(), "Segmentation fault");
        assert_eq!(FailureSignal::SigAbrt.description(), "Abort");
        assert_eq!(FailureSignal::SigIll.description(), "Illegal instruction");
        assert_eq!(FailureSignal::SigFpe.description(), "Floating point exception");
        assert_eq!(FailureSignal::SigBus.description(), "Bus error");
        assert_eq!(FailureSignal::SigTerm.description(), "Termination");
        assert_eq!(FailureSignal::SigInt.description(), "Interrupt");
        assert_eq!(FailureSignal::Unknown(42).description(), "Unknown signal");
    }

    #[test]
    fn test_failure_signal_display() {
        let sig = FailureSignal::SigSegv;
        let s = format!("{}", sig);
        assert!(s.contains("SIGSEGV"));
        assert!(s.contains("Segmentation fault"));
    }

    #[test]
    fn test_print_failure_handler() {
        let handler = PrintFailureHandler;
        let ctx = FailureContext::new(FailureSignal::SigSegv);
        // Just verify it doesn't panic
        handler.handle_signal(FailureSignal::SigSegv, &ctx.backtrace);
    }

    #[test]
    fn test_extended_signal_name() {
        assert_eq!(ExtendedSignal::StackOverflow.name(), "STACK_OVERFLOW");
        assert_eq!(ExtendedSignal::HeapCorruption.name(), "HEAP_CORRUPTION");
        assert_eq!(ExtendedSignal::DataRace.name(), "DATA_RACE");
    }

    #[test]
    fn test_extended_signal_description() {
        assert_eq!(ExtendedSignal::StackOverflow.description(), "Stack overflow detected");
        assert_eq!(ExtendedSignal::Deadlock.description(), "Deadlock detected");
    }

    #[test]
    fn test_extended_signal_display() {
        let sig = ExtendedSignal::UseAfterFree;
        let s = format!("{}", sig);
        assert!(s.contains("USE_AFTER_FREE"));
    }

    #[test]
    fn test_register_state_new() {
        let regs = RegisterState::new();
        assert_eq!(regs.ip, 0);
        assert_eq!(regs.sp, 0);
        assert!(regs.fp.is_none());
    }

    #[test]
    fn test_register_state_with_ip() {
        let regs = RegisterState::new().with_ip(0x1000);
        assert_eq!(regs.ip, 0x1000);
    }

    #[test]
    fn test_register_state_with_sp() {
        let regs = RegisterState::new().with_sp(0x2000);
        assert_eq!(regs.sp, 0x2000);
    }

    #[test]
    fn test_register_state_with_fp() {
        let regs = RegisterState::new().with_fp(0x3000);
        assert_eq!(regs.fp, Some(0x3000));
    }

    #[test]
    fn test_failure_context_new() {
        let ctx = FailureContext::new(FailureSignal::SigSegv);
        assert_eq!(ctx.signal, FailureSignal::SigSegv);
    }

    #[test]
    fn test_failure_context_with_description() {
        let ctx = FailureContext::new(FailureSignal::SigSegv)
            .with_description("Test failure".to_string());
        assert_eq!(ctx.description, "Test failure");
    }

    #[test]
    fn test_failure_context_display() {
        let ctx = FailureContext::new(FailureSignal::SigSegv);
        let s = format!("{}", ctx);
        assert!(s.contains("SIGSEGV"));
    }
}
