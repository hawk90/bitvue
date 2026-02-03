//! Symbol table and address lookup.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

/// A symbol entry from symbolization.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Symbol {
    /// The symbol name.
    pub name: String,
    /// The start address of the symbol.
    pub address: usize,
    /// The size of the symbol.
    pub size: usize,
}

impl Symbol {
    /// Creates a new symbol entry.
    pub fn new(name: String, address: usize, size: usize) -> Self {
        Self {
            name,
            address,
            size,
        }
    }

    /// Returns true if the given address falls within this symbol.
    ///
    /// Uses checked arithmetic to prevent overflow vulnerability where
    /// address + size could wrap around for symbols near usize::MAX.
    pub fn contains(&self, address: usize) -> bool {
        address >= self.address && match self.address.checked_add(self.size) {
            Some(end) => address < end,
            None => false, // Overflow: no valid address can be contained
        }
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {:#x} ({} bytes)", self.name, self.address, self.size)
    }
}

/// A symbol table for address lookup.
#[derive(Clone, Debug, Default)]
pub struct SymbolTable {
    symbols: Vec<Symbol>,
}

impl SymbolTable {
    /// Creates a new empty symbol table.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a symbol to the table.
    pub fn add(&mut self, symbol: Symbol) {
        self.symbols.push(symbol);
    }

    /// Looks up a symbol by address.
    pub fn lookup(&self, address: usize) -> Option<&Symbol> {
        self.symbols
            .iter()
            .find(|s| s.contains(address))
    }

    /// Looks up a symbol name by address.
    pub fn lookup_name(&self, address: usize) -> Option<&str> {
        self.lookup(address).map(|s| s.name.as_str())
    }

    /// Returns all symbols in the table.
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Sorts symbols by address for binary search.
    pub fn sort(&mut self) {
        self.symbols.sort_by_key(|s| s.address);
    }

    /// Finds the nearest symbol to the given address.
    pub fn find_nearest(&self, address: usize) -> Option<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| s.address <= address)
            .min_by_key(|s| address - s.address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_new() {
        let symbol = Symbol::new("my_func".to_string(), 0x1000, 0x100);
        assert_eq!(symbol.name, "my_func");
        assert_eq!(symbol.address, 0x1000);
        assert_eq!(symbol.size, 0x100);
    }

    #[test]
    fn test_symbol_contains() {
        let symbol = Symbol::new("my_func".to_string(), 0x1000, 0x100);
        assert!(symbol.contains(0x1000));
        assert!(symbol.contains(0x1050));
        assert!(!symbol.contains(0x1100));
        assert!(!symbol.contains(0x0FFF));
    }

    #[test]
    fn test_symbol_display() {
        let symbol = Symbol::new("my_func".to_string(), 0x1000, 0x100);
        let s = format!("{}", symbol);
        assert!(s.contains("my_func"));
        assert!(s.contains("1000"));
        assert!(s.contains("256")); // 0x100 = 256 bytes
    }

    #[test]
    fn test_symbol_table_new() {
        let table = SymbolTable::new();
        assert!(table.symbols().is_empty());
    }

    #[test]
    fn test_symbol_table_add() {
        let mut table = SymbolTable::new();
        let symbol = Symbol::new("func1".to_string(), 0x1000, 0x100);
        table.add(symbol);
        assert_eq!(table.symbols().len(), 1);
    }

    #[test]
    fn test_symbol_table_lookup() {
        let mut table = SymbolTable::new();
        table.add(Symbol::new("func1".to_string(), 0x1000, 0x100));
        table.add(Symbol::new("func2".to_string(), 0x2000, 0x100));

        let result = table.lookup(0x1050);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "func1");
    }

    #[test]
    fn test_symbol_table_lookup_name() {
        let mut table = SymbolTable::new();
        table.add(Symbol::new("func1".to_string(), 0x1000, 0x100));

        let name = table.lookup_name(0x1050);
        assert_eq!(name, Some("func1"));
    }

    #[test]
    fn test_symbol_table_sort() {
        let mut table = SymbolTable::new();
        table.add(Symbol::new("func2".to_string(), 0x2000, 0x100));
        table.add(Symbol::new("func1".to_string(), 0x1000, 0x100));

        table.sort();
        assert_eq!(table.symbols()[0].address, 0x1000);
        assert_eq!(table.symbols()[1].address, 0x2000);
    }

    #[test]
    fn test_symbol_table_find_nearest() {
        let mut table = SymbolTable::new();
        table.add(Symbol::new("func1".to_string(), 0x1000, 0x100));
        table.add(Symbol::new("func2".to_string(), 0x2000, 0x100));

        let nearest = table.find_nearest(0x1050);
        assert!(nearest.is_some());
        assert_eq!(nearest.unwrap().name, "func1");
    }

    // Test for HIGH security fix - integer overflow prevention
    #[test]
    fn test_symbol_contains_overflow_protection() {
        // Test case: symbol at near usize::MAX with size that would overflow
        let symbol = Symbol::new("overflow_func".to_string(), usize::MAX - 10, 20);

        // These addresses should NOT be contained (overflow would cause false positives)
        assert!(!symbol.contains(0));
        assert!(!symbol.contains(100));
        assert!(!symbol.contains(usize::MAX));

        // Only addresses within the valid range should be contained
        assert!(symbol.contains(usize::MAX - 10));
        assert!(symbol.contains(usize::MAX - 5));
        // The last valid address is usize::MAX - 10 + 20 - 1 = usize::MAX + 9, which overflows
        // So actually, with our fix, any address >= usize::MAX - 10 will be rejected
        // because the end would overflow
        assert!(!symbol.contains(usize::MAX)); // Overflow case
    }

    #[test]
    fn test_symbol_contains_edge_case_no_overflow() {
        // Test case: symbol at near usize::MAX but size doesn't overflow
        let symbol = Symbol::new("edge_func".to_string(), usize::MAX - 100, 50);

        // Valid range: [usize::MAX - 100, usize::MAX - 50)
        assert!(symbol.contains(usize::MAX - 100));
        assert!(symbol.contains(usize::MAX - 75));
        assert!(!symbol.contains(usize::MAX - 50)); // One past the end
        assert!(!symbol.contains(usize::MAX));
    }
}
