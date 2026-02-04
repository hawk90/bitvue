//! Breakpoint and watchpoint management.

use alloc::string::String;
use alloc::vec::Vec;

/// A debugging breakpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Breakpoint {
    /// The address of the breakpoint.
    pub address: usize,
    /// Whether the breakpoint is enabled.
    pub enabled: bool,
    /// A condition for the breakpoint (optional).
    pub condition: Option<String>,
}

impl Breakpoint {
    /// Creates a new breakpoint.
    pub fn new(address: usize) -> Self {
        Self {
            address,
            enabled: true,
            condition: None,
        }
    }

    /// Sets the enabled state.
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets a condition for the breakpoint.
    pub fn with_condition(mut self, condition: String) -> Self {
        self.condition = Some(condition);
        self
    }
}

/// A breakpoint manager.
#[derive(Clone, Debug, Default)]
pub struct BreakpointManager {
    breakpoints: Vec<Breakpoint>,
}

impl BreakpointManager {
    /// Creates a new breakpoint manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a breakpoint.
    pub fn add(&mut self, breakpoint: Breakpoint) {
        self.breakpoints.push(breakpoint);
    }

    /// Removes a breakpoint at the given address.
    pub fn remove(&mut self, address: usize) -> bool {
        let pos = self.breakpoints.iter().position(|b| b.address == address);
        if let Some(pos) = pos {
            self.breakpoints.remove(pos);
            true
        } else {
            false
        }
    }

    /// Returns all breakpoints.
    pub fn breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    /// Returns enabled breakpoints.
    pub fn enabled_breakpoints(&self) -> Vec<&Breakpoint> {
        self.breakpoints
            .iter()
            .filter(|b| b.enabled)
            .collect()
    }

    /// Finds a breakpoint at the given address.
    pub fn find(&self, address: usize) -> Option<&Breakpoint> {
        self.breakpoints.iter().find(|b| b.address == address)
    }

    /// Clears all breakpoints.
    pub fn clear(&mut self) {
        self.breakpoints.clear();
    }
}

/// Watchpoint type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WatchpointType {
    /// Break on write.
    Write,
    /// Break on read.
    Read,
    /// Break on read or write.
    ReadWrite,
}

/// A watchpoint for monitoring memory accesses.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Watchpoint {
    /// The address being watched.
    pub address: usize,
    /// The size of the region being watched.
    pub size: usize,
    /// The watchpoint type.
    pub watch_type: WatchpointType,
    /// Whether the watchpoint is enabled.
    pub enabled: bool,
}

impl Watchpoint {
    /// Creates a new watchpoint.
    pub fn new(address: usize, size: usize, watch_type: WatchpointType) -> Self {
        Self {
            address,
            size,
            watch_type,
            enabled: true,
        }
    }

    /// Returns true if the given address/size intersects with this watchpoint.
    pub fn matches(&self, address: usize, size: usize) -> bool {
        let self_end = self.address.saturating_add(self.size);
        let other_end = address.saturating_add(size);
        !(other_end <= self.address || address >= self_end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_breakpoint_new() {
        let bp = Breakpoint::new(0x1000);
        assert_eq!(bp.address, 0x1000);
        assert!(bp.enabled);
        assert!(bp.condition.is_none());
    }

    #[test]
    fn test_breakpoint_with_enabled() {
        let bp = Breakpoint::new(0x1000).with_enabled(false);
        assert!(!bp.enabled);
    }

    #[test]
    fn test_breakpoint_with_condition() {
        let bp = Breakpoint::new(0x1000)
            .with_condition("x > 0".to_string());
        assert_eq!(bp.condition, Some("x > 0".to_string()));
    }

    #[test]
    fn test_breakpoint_manager_new() {
        let mgr = BreakpointManager::new();
        assert!(mgr.breakpoints().is_empty());
    }

    #[test]
    fn test_breakpoint_manager_add() {
        let mut mgr = BreakpointManager::new();
        mgr.add(Breakpoint::new(0x1000));
        assert_eq!(mgr.breakpoints().len(), 1);
    }

    #[test]
    fn test_breakpoint_manager_remove() {
        let mut mgr = BreakpointManager::new();
        mgr.add(Breakpoint::new(0x1000));
        assert!(mgr.remove(0x1000));
        assert!(mgr.breakpoints().is_empty());
    }

    #[test]
    fn test_breakpoint_manager_remove_nonexistent() {
        let mut mgr = BreakpointManager::new();
        assert!(!mgr.remove(0x1000));
    }

    #[test]
    fn test_breakpoint_manager_find() {
        let mut mgr = BreakpointManager::new();
        mgr.add(Breakpoint::new(0x1000));
        let bp = mgr.find(0x1000);
        assert!(bp.is_some());
        assert_eq!(bp.unwrap().address, 0x1000);
    }

    #[test]
    fn test_breakpoint_manager_enabled_breakpoints() {
        let mut mgr = BreakpointManager::new();
        mgr.add(Breakpoint::new(0x1000).with_enabled(false));
        mgr.add(Breakpoint::new(0x2000).with_enabled(true));

        let enabled = mgr.enabled_breakpoints();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].address, 0x2000);
    }

    #[test]
    fn test_breakpoint_manager_clear() {
        let mut mgr = BreakpointManager::new();
        mgr.add(Breakpoint::new(0x1000));
        mgr.add(Breakpoint::new(0x2000));
        mgr.clear();
        assert!(mgr.breakpoints().is_empty());
    }

    #[test]
    fn test_watchpoint_new() {
        let wp = Watchpoint::new(0x1000, 8, WatchpointType::Write);
        assert_eq!(wp.address, 0x1000);
        assert_eq!(wp.size, 8);
        assert!(wp.enabled);
    }

    #[test]
    fn test_watchpoint_matches() {
        let wp = Watchpoint::new(0x1000, 16, WatchpointType::ReadWrite);
        assert!(wp.matches(0x1000, 4));
        assert!(wp.matches(0x1008, 4));
        assert!(!wp.matches(0x1010, 4));
    }

    #[test]
    fn test_watchpoint_type_variants() {
        let _ = WatchpointType::Read;
        let _ = WatchpointType::Write;
        let _ = WatchpointType::ReadWrite;
    }
}
