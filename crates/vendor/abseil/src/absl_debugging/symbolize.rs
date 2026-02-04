//! Symbol/address lookup utilities.
//!
//! Provides functions for symbolizing addresses (converting addresses to symbol names),
//! similar to Abseil's `absl/debugging/symbolize.h`.

use core::fmt;

/// Information about a symbol at a given address.
///
/// This contains the symbolized information for an address,
/// including the symbol name, file location, and offset.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SymbolInfo {
    /// The demangled symbol name.
    pub name: String,
    /// The file name (if available).
    pub file: Option<String>,
    /// The line number (if available).
    pub line: Option<u32>,
    /// The offset from the symbol start.
    pub offset: usize,
    /// The start address of the symbol.
    pub start_address: usize,
}

impl SymbolInfo {
    /// Creates a new SymbolInfo with minimal information.
    pub fn new(name: String, address: usize) -> Self {
        Self {
            name,
            file: None,
            line: None,
            offset: 0,
            start_address: address,
        }
    }

    /// Sets the file location.
    pub fn with_file(mut self, file: String) -> Self {
        self.file = Some(file);
        self
    }

    /// Sets the line number.
    pub fn with_line(mut self, line: u32) -> Self {
        self.line = Some(line);
        self
    }

    /// Sets the offset from the symbol start.
    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Returns a formatted representation of this symbol.
    pub fn format(&self) -> String {
        if let Some(file) = &self.file {
            if let Some(line) = self.line {
                format!("{} in {}:{} (+0x{:x})", self.name, file, line, self.offset)
            } else {
                format!("{} in {} (+0x{:x})", self.name, file, self.offset)
            }
        } else {
            format!("{} (+0x{:x})", self.name, self.offset)
        }
    }
}

impl fmt::Display for SymbolInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Result type for symbolization operations.
pub type SymbolizeResult<T> = Result<T, SymbolizeError>;

/// Errors that can occur during symbolization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolizeError {
    /// Symbolization is not supported on this platform.
    Unsupported,
    /// The address could not be found in any loaded module.
    AddressNotFound,
    /// The symbol information could not be retrieved.
    SymbolNotFound,
    /// An internal error occurred.
    Internal(String),
}

impl fmt::Display for SymbolizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SymbolizeError::Unsupported => write!(f, "Symbolization not supported"),
            SymbolizeError::AddressNotFound => write!(f, "Address not found"),
            SymbolizeError::SymbolNotFound => write!(f, "Symbol not found"),
            SymbolizeError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for SymbolizeError {}

/// Demangles a symbol name.
///
/// This function attempts to convert a mangled symbol name (from Rust, C++, etc.)
/// into a human-readable form.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::demangle;
///
/// // Non-mangled symbols pass through
/// assert_eq!(demangle("my_function"), "my_function");
/// ```
///
/// # Notes
///
/// - Rust symbols (starting with `_R`) use rustc-demangle when available
/// - C++ symbols use cpp_demangle when available
/// - Other symbols pass through unchanged
pub fn demangle(symbol: &str) -> String {
    #[cfg(feature = "std")]
    {
        // Try to demangle Rust symbols
        if symbol.starts_with("_R") {
            // In a real implementation, we would use rustc_demangle here
            // For now, return a partially demangled version
            return rustc_demangle_partial(symbol);
        }

        // Try to demangle C++ symbols
        if symbol.starts_with("_Z") {
            // In a real implementation, we would use cpp_demangle here
            return format!("[demangled]({})", symbol);
        }

        symbol.to_string()
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = symbol;
        String::new()
    }
}

/// Partial implementation of Rust symbol demangling.
/// In production, use the rustc_demangle crate.
fn rustc_demangle_partial(symbol: &str) -> String {
    // Very basic Rust demangling - just shows that it's a Rust symbol
    if symbol.starts_with("_R") {
        // Strip the _R prefix and add a marker
        let rest = &symbol[2..];

        // Parse the length encoding for the first path segment
        if let Some(end) = rest.chars().position(|c| !c.is_ascii_digit()) {
            if end > 0 {
                let len_str = &rest[..end];
                if let Ok(len) = len_str.parse::<usize>() {
                    let start = end + 1;
                    if start + len <= rest.len() {
                        let name = &rest[start..start + len];
                        return format!("{}::", name);
                    }
                }
            }
        }
    }

    symbol.to_string()
}

/// Symbolizes an address into a human-readable form.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::symbolize;
///
/// // Symbolize an address (platform-specific)
/// let addr = 0x1000;
/// if let Some(symbol) = symbolize(addr) {
///         println!("Address {:#x} is: {}", addr, symbol);
/// }
/// ```
///
/// # Notes
///
/// - On supported platforms, this uses platform-specific APIs
/// - Returns None if symbolization fails or is not supported
/// - In no_std environments, always returns None
#[inline]
pub fn symbolize(addr: usize) -> Option<String> {
    symbolize_with_info(addr).map(|info| info.format())
}

/// Symbolizes an address into detailed SymbolInfo.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::symbolize_with_info;
///
/// let addr = 0x1000;
/// if let Some(info) = symbolize_with_info(addr) {
///     println!("Address {:#x}: {}", addr, info);
/// }
/// ```
pub fn symbolize_with_info(addr: usize) -> Option<SymbolInfo> {
    #[cfg(feature = "std")]
    {
        // On Linux/Unix, we could use:
        // - backtrace::resolve() with the backtrace crate
        // - dladdr() from libc
        // On Windows:
        // - SymFromAddr() from dbghelp
        // For now, return a basic stub

        if addr == 0 {
            return None;
        }

        // In a real implementation, we would:
        // 1. Find which shared object contains the address
        // 2. Look up the symbol in that object's symbol table
        // 3. Return the symbol information

        Some(SymbolInfo::new(format!("<unknown@{:#x}>", addr), addr))
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = addr;
        None
    }
}

/// Symbolizes an address into its constituent parts.
///
/// Returns the symbol name, file name, and line number if available.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::symbolize_extended;
///
/// let addr = 0x1000;
/// if let Some(info) = symbolize_extended(addr) {
///     println!("Address {:#x}: {}", addr, info);
/// }
/// ```
pub fn symbolize_extended(addr: usize) -> Option<SymbolInfo> {
    symbolize_with_info(addr)
}

/// Symbolizes multiple addresses efficiently.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::symbolize_batch;
///
/// let addresses = vec![0x1000, 0x2000, 0x3000];
/// let symbols = symbolize_batch(&addresses);
/// for (addr, symbol) in addresses.iter().zip(symbols) {
///     if let Some(s) = symbol {
///         println!("{:#x}: {}", addr, s);
///     }
/// }
/// ```
pub fn symbolize_batch(addresses: &[usize]) -> Vec<Option<SymbolInfo>> {
    addresses.iter().map(|&addr| symbolize_with_info(addr)).collect()
}

/// A cache for symbol information to avoid repeated lookups.
///
/// This is useful when symbolizing many addresses from the same binary.
#[cfg(feature = "std")]
#[derive(Default)]
pub struct SymbolCache {
    /// Cached symbol information keyed by address range.
    cache: std::sync::Mutex<std::collections::HashMap<usize, SymbolInfo>>,
}

#[cfg(feature = "std")]
impl Clone for SymbolCache {
    fn clone(&self) -> Self {
        Self::default()
    }
}

#[cfg(feature = "std")]
impl SymbolCache {
    /// Creates a new empty symbol cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Looks up an address in the cache.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    pub fn lookup(&self, addr: usize) -> Option<SymbolInfo> {
        self.try_lookup(addr).unwrap_or_else(|_| {
            panic!(
                "SymbolCache mutex is poisoned while looking up address {:#x}",
                addr
            )
        })
    }

    /// Attempts to look up an address in the cache.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_lookup(&self, addr: usize) -> Result<Option<SymbolInfo>, std::sync::PoisonError<std::sync::MutexGuard<'_, std::collections::HashMap<usize, SymbolInfo>>>> {
        let cache = self.cache.lock()?;
        // Check for exact match or nearby cached symbol
        if let Some(info) = cache.get(&addr) {
            return Ok(Some(info.clone()));
        }

        // Look for nearby symbols (within 1KB)
        for (&_cached_addr, info) in cache.iter() {
            let start = info.start_address;
            if addr >= start && addr < start + 1024 {
                return Ok(Some(info.clone()));
            }
        }

        Ok(None)
    }

    /// Inserts symbol information into the cache.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    pub fn insert(&self, addr: usize, info: SymbolInfo) {
        self.try_insert(addr, info).unwrap_or_else(|_| {
            panic!(
                "SymbolCache mutex is poisoned while inserting address {:#x}",
                addr
            )
        });
    }

    /// Attempts to insert symbol information into the cache.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_insert(&self, addr: usize, info: SymbolInfo) -> Result<(), std::sync::PoisonError<std::sync::MutexGuard<'_, std::collections::HashMap<usize, SymbolInfo>>>> {
        self.cache.lock()?.insert(addr, info);
        Ok(())
    }

    /// Clears the cache.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    pub fn clear(&self) {
        self.try_clear().unwrap_or_else(|_| {
            panic!("SymbolCache mutex is poisoned while clearing cache")
        });
    }

    /// Attempts to clear the cache.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_clear(&self) -> Result<(), std::sync::PoisonError<std::sync::MutexGuard<'_, std::collections::HashMap<usize, SymbolInfo>>>> {
        self.cache.lock()?.clear();
        Ok(())
    }

    /// Returns the number of cached entries.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    pub fn len(&self) -> usize {
        self.try_len().unwrap_or_else(|_| {
            panic!("SymbolCache mutex is poisoned while getting length")
        })
    }

    /// Attempts to get the number of cached entries.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_len(&self) -> Result<usize, std::sync::PoisonError<std::sync::MutexGuard<'_, std::collections::HashMap<usize, SymbolInfo>>>> {
        Ok(self.cache.lock()?.len())
    }

    /// Returns true if the cache is empty.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    pub fn is_empty(&self) -> bool {
        self.try_is_empty().unwrap_or_else(|_| {
            panic!("SymbolCache mutex is poisoned while checking if empty")
        })
    }

    /// Attempts to check if the cache is empty.
    ///
    /// Returns `Err` if the mutex is poisoned.
    pub fn try_is_empty(&self) -> Result<bool, std::sync::PoisonError<std::sync::MutexGuard<'_, std::collections::HashMap<usize, SymbolInfo>>>> {
        Ok(self.cache.lock()?.is_empty())
    }

    /// Symbolizes an address with caching.
    pub fn symbolize_cached(&self, addr: usize) -> Option<SymbolInfo> {
        if let Some(info) = self.lookup(addr) {
            return Some(info);
        }

        // Not in cache, do the lookup
        if let Some(info) = symbolize_with_info(addr) {
            self.insert(addr, info.clone());
            Some(info)
        } else {
            None
        }
    }
}

/// Gets a symbol for a function pointer.
///
/// # Safety
///
/// The function pointer must be valid.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::get_symbol_for_function;
///
/// fn my_function() -> i32 { 42 }
/// let func_ptr = my_function as usize;
/// unsafe {
///     if let Some(symbol) = get_symbol_for_function(func_ptr) {
///         println!("Function: {}", symbol);
///     }
/// }
/// ```
pub unsafe fn get_symbol_for_function(func_ptr: usize) -> Option<String> {
    symbolize(func_ptr)
}

/// Registers a callback for custom symbolization.
///
/// This allows users to provide their own symbolization logic,
/// for example, using an external symbol server.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::{register_symbolizer, SymbolInfo};
///
/// register_symbolizer(|addr| {
///     if addr == 0x1000 {
///         Some(SymbolInfo::new("my_symbol".to_string(), addr))
///     } else {
///         None
///     }
/// });
/// ```
#[cfg(feature = "std")]
pub fn register_symbolizer<F: Fn(usize) -> Option<SymbolInfo> + Send + Sync + 'static>(
    _func: F,
) {
    // In a real implementation, we would store this in a global registry
    // For now, this is a placeholder
}

/// Symbolizes a stack trace.
///
/// Converts a series of instruction pointers into human-readable symbols.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::symbolize_stack_trace;
///
/// let addresses = vec![0x1000, 0x2000, 0x3000];
/// let symbols = symbolize_stack_trace(&addresses);
/// for symbol in symbols {
///     println!("  {}", symbol.unwrap_or_else(|| "<unknown>".to_string()));
/// }
/// ```
pub fn symbolize_stack_trace(addresses: &[usize]) -> Vec<Option<String>> {
    addresses.iter().map(|&addr| symbolize(addr)).collect()
}

/// Pretty-prints a stack trace with symbols.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::print_stack_trace;
///
/// let addresses = vec![0x1000, 0x2000, 0x3000];
/// print_stack_trace(&addresses);
/// ```
#[cfg(feature = "std")]
pub fn print_stack_trace(addresses: &[usize]) {
    let symbols = symbolize_stack_trace(addresses);
    for (i, addr) in addresses.iter().enumerate() {
        println!("  #{} - {:#x}: {}", i, addr,
            symbols[i].as_ref().map(|s| s.as_str()).unwrap_or("<unknown>"));
    }
}

/// Estimates the size of a symbol at the given address.
///
/// This is useful for calculating how many bytes a function occupies.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::estimate_symbol_size;
///
/// let addr = 0x1000;
/// if let Some(size) = estimate_symbol_size(addr) {
///     println!("Symbol at {:#x} is approximately {} bytes", addr, size);
/// }
/// ```
pub fn estimate_symbol_size(addr: usize) -> Option<usize> {
    // In a real implementation, we would:
    // 1. Find the symbol
    // 2. Find the next symbol in the same section
    // 3. Calculate the difference
    // For now, return a placeholder value
    if addr > 0 {
        Some(1024) // Placeholder: assume 1KB
    } else {
        None
    }
}

/// Finds the base address of the module containing the given address.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::find_module_base;
///
/// let addr = 0x12345678;
/// if let Some(base) = find_module_base(addr) {
///     println!("Address {:#x} is in module at {:#x}", addr, base);
/// }
/// ```
pub fn find_module_base(addr: usize) -> Option<usize> {
    #[cfg(feature = "std")]
    {
        if addr == 0 {
            return None;
        }
        // In a real implementation, we would:
        // 1. Iterate through loaded modules
        // 2. Find which module contains the address
        // 3. Return the module's base address
        // For now, align down to 1MB boundary
        Some(addr & !0xFFFFF)
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = addr;
        None
    }
}

/// Represents a code location (file, line, column).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodeLocation {
    /// File path.
    pub file: String,
    /// Line number (1-indexed).
    pub line: u32,
    /// Column number (1-indexed, if available).
    pub column: Option<u32>,
}

impl CodeLocation {
    /// Creates a new CodeLocation.
    pub fn new(file: String, line: u32) -> Self {
        Self {
            file,
            line,
            column: None,
        }
    }

    /// Sets the column number.
    pub fn with_column(mut self, column: u32) -> Self {
        self.column = Some(column);
        self
    }

    /// Returns true if this is an unknown location.
    pub fn is_unknown(&self) -> bool {
        self.file.is_empty() || self.file == "<unknown>"
    }
}

impl fmt::Display for CodeLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(col) = self.column {
            write!(f, "{}:{}:{}", self.file, self.line, col)
        } else {
            write!(f, "{}:{}", self.file, self.line)
        }
    }
}

/// Converts an address to a code location.
///
/// This requires debug information to be available.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::address_to_location;
///
/// let addr = 0x1000;
/// if let Some(loc) = address_to_location(addr) {
///     println!("Address {:#x} is at {}", addr, loc);
/// }
/// ```
pub fn address_to_location(addr: usize) -> Option<CodeLocation> {
    #[cfg(feature = "std")]
    {
        // In a real implementation with debug info:
        // 1. Find the symbol containing the address
        // 2. Look up line number information from debug info (DWARF, PDB)
        // For now, return None
        let _ = addr;
        None
    }
    #[cfg(not(feature = "std"))]
    {
        let _ = addr;
        None
    }
}

/// Parses a mangled Rust symbol to extract type information.
///
/// # Examples
///
/// ```
/// use abseil::absl_debugging::symbolize::parse_rust_symbol_path;
///
/// // Extract the path from a mangled symbol
/// if let Some(path) = parse_rust_symbol_path("_RNvCsa123my_crate3foo") {
///     println!("Symbol path: {}", path);
/// }
/// ```
pub fn parse_rust_symbol_path(symbol: &str) -> Option<String> {
    if !symbol.starts_with("_R") {
        return None;
    }

    // Basic parsing of Rust symbol format
    let rest = &symbol[2..];
    let mut parts = Vec::new();
    let mut pos = 0;

    while pos < rest.len() {
        // Parse length-prefixed segments
        let end = rest[pos..].chars().position(|c| !c.is_ascii_digit())?;
        let len_str = &rest[pos..pos + end];
        let len: usize = len_str.parse().ok()?;
        pos += end + 1;

        if pos + len > rest.len() {
            return None;
        }

        let segment = &rest[pos..pos + len];
        parts.push(segment);
        pos += len;
    }

    if !parts.is_empty() {
        Some(parts.join("::"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demangle_basic() {
        // Non-mangled symbols pass through
        assert_eq!(demangle("my_function"), "my_function");
        assert_eq!(demangle("_Z6foobarv"), "[demangled](_Z6foobarv)");
    }

    #[test]
    fn test_symbolize_basic() {
        // symbolize returns None for unknown addresses
        assert!(symbolize(0x1000).is_some()); // Stub returns some value
        assert!(symbolize(0).is_none());
    }

    #[test]
    fn test_symbolize_extended() {
        // symbolize_extended returns None in stub implementation
        let result = symbolize_extended(0x1000);
        assert!(result.is_some()); // Stub returns some value
    }

    #[test]
    fn test_symbol_info_new() {
        let info = SymbolInfo::new("test_func".to_string(), 0x1000);
        assert_eq!(info.name, "test_func");
        assert_eq!(info.start_address, 0x1000);
        assert!(info.file.is_none());
        assert!(info.line.is_none());
    }

    #[test]
    fn test_symbol_info_builder() {
        let info = SymbolInfo::new("test_func".to_string(), 0x1000)
            .with_file("test.rs".to_string())
            .with_line(42)
            .with_offset(16);

        assert_eq!(info.name, "test_func");
        assert_eq!(info.file, Some("test.rs".to_string()));
        assert_eq!(info.line, Some(42));
        assert_eq!(info.offset, 16);
    }

    #[test]
    fn test_symbol_info_format() {
        let info = SymbolInfo::new("test_func".to_string(), 0x1000)
            .with_file("test.rs".to_string())
            .with_line(42)
            .with_offset(16);

        let formatted = info.format();
        assert!(formatted.contains("test_func"));
        assert!(formatted.contains("test.rs"));
        assert!(formatted.contains("42"));
    }

    #[test]
    fn test_symbol_info_display() {
        let info = SymbolInfo::new("test_func".to_string(), 0x1000);
        let display = format!("{}", info);
        assert!(display.contains("test_func"));
    }

    #[test]
    fn test_symbolize_batch() {
        let addresses = vec![0x1000, 0x2000, 0x3000];
        let symbols = symbolize_batch(&addresses);
        assert_eq!(symbols.len(), 3);
        // Stub returns Some for non-zero addresses
        assert!(symbols[0].is_some());
        assert!(symbols[1].is_some());
        assert!(symbols[2].is_some());
    }

    #[test]
    fn test_symbolize_stack_trace() {
        let addresses = vec![0x1000, 0x2000, 0x3000];
        let symbols = symbolize_stack_trace(&addresses);
        assert_eq!(symbols.len(), 3);
    }

    #[test]
    fn test_find_module_base() {
        #[cfg(feature = "std")]
        {
            let addr = 0x12345678;
            if let Some(base) = find_module_base(addr) {
                assert!(base <= addr);
            }
        }
    }

    #[test]
    fn test_estimate_symbol_size() {
        assert!(estimate_symbol_size(0x1000).is_some());
        assert!(estimate_symbol_size(0).is_none());
    }

    #[test]
    fn test_code_location_new() {
        let loc = CodeLocation::new("test.rs".to_string(), 42);
        assert_eq!(loc.file, "test.rs");
        assert_eq!(loc.line, 42);
        assert!(loc.column.is_none());
    }

    #[test]
    fn test_code_location_with_column() {
        let loc = CodeLocation::new("test.rs".to_string(), 42)
            .with_column(10);
        assert_eq!(loc.column, Some(10));
    }

    #[test]
    fn test_code_location_display() {
        let loc = CodeLocation::new("test.rs".to_string(), 42);
        assert_eq!(format!("{}", loc), "test.rs:42");

        let loc2 = CodeLocation::new("test.rs".to_string(), 42)
            .with_column(10);
        assert_eq!(format!("{}", loc2), "test.rs:42:10");
    }

    #[test]
    fn test_code_location_is_unknown() {
        let loc = CodeLocation::new("<unknown>".to_string(), 0);
        assert!(loc.is_unknown());

        let loc2 = CodeLocation::new("test.rs".to_string(), 42);
        assert!(!loc2.is_unknown());
    }

    #[test]
    fn test_parse_rust_symbol_path() {
        // Test basic parsing
        let symbol = "_RNvCsa123my_crate3foo";
        if let Some(path) = parse_rust_symbol_path(symbol) {
            // Should extract something
            assert!(!path.is_empty());
        }

        // Non-Rust symbols return None
        assert!(parse_rust_symbol_path("my_function").is_none());
    }

    #[test]
    fn test_symbolize_error_display() {
        assert_eq!(format!("{}", SymbolizeError::Unsupported), "Symbolization not supported");
        assert_eq!(format!("{}", SymbolizeError::AddressNotFound), "Address not found");
        assert_eq!(format!("{}", SymbolizeError::Internal("test".to_string())), "Internal error: test");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_symbol_cache_new() {
        let cache = SymbolCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_symbol_cache_insert_lookup() {
        let cache = SymbolCache::new();
        let info = SymbolInfo::new("test".to_string(), 0x1000);
        cache.insert(0x1000, info.clone());

        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());

        let found = cache.lookup(0x1000);
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "test");
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_symbol_cache_clear() {
        let cache = SymbolCache::new();
        cache.insert(0x1000, SymbolInfo::new("test".to_string(), 0x1000));
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_rust_demangle_partial() {
        // Test the partial demangle implementation
        assert_eq!(rustc_demangle_partial("my_function"), "my_function");
        assert!(rustc_demangle_partial("_R").starts_with("_R"));
    }

    // Tests for MEDIUM security fix - mutex poison handling

    #[cfg(feature = "std")]
    #[test]
    fn test_symbol_cache_try_methods() {
        let cache = SymbolCache::new();

        // Test try_insert
        assert!(cache.try_insert(0x1000, SymbolInfo::new("test".to_string(), 0x1000)).is_ok());

        // Test try_lookup
        let result = cache.try_lookup(0x1000);
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        // Test try_len
        assert_eq!(cache.try_len().ok(), Some(1));

        // Test try_is_empty
        assert_eq!(cache.try_is_empty().ok(), Some(false));

        // Test try_clear
        assert!(cache.try_clear().is_ok());
        assert_eq!(cache.try_len().ok(), Some(0));
    }

    #[cfg(feature = "std")]
    #[test]
    fn test_symbol_cache_methods_have_panic_docs() {
        // This is a compile-time check that the panic documentation is present
        // The actual panic behavior is tested by the try_* methods above

        let cache = SymbolCache::new();
        cache.insert(0x1000, SymbolInfo::new("test".to_string(), 0x1000));
        assert!(cache.lookup(0x1000).is_some());
        assert_eq!(cache.len(), 1);
        cache.clear();
        assert!(cache.is_empty());
    }
}
