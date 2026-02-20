//! Resource budget tracking for preventing gradual resource exhaustion
//!
//! This module provides a mechanism to track and limit cumulative allocations
//! across parsing operations to prevent DoS attacks via many small allocations.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Maximum cumulative allocation per session (500 MB)
///
/// This prevents attackers from gradually exhausting memory through
/// many small allocations across multiple parsing operations.
const MAX_CUMULATIVE_ALLOCATION: u64 = 500 * 1024 * 1024;

/// Per-operation allocation limit (50 MB)
///
/// Limits the size of any single parsing operation.
const MAX_SINGLE_ALLOCATION: u64 = 50 * 1024 * 1024;

/// Resource budget tracker
///
/// Tracks cumulative allocations across parsing operations with thread-safe
/// atomic counters. Cloning this handle shares the budget with the new owner.
#[derive(Debug, Clone)]
pub struct ResourceBudget {
    /// Shared allocation counter
    allocated: Arc<AtomicU64>,
}

impl ResourceBudget {
    /// Create a new resource budget with no allocations yet
    pub fn new() -> Self {
        Self {
            allocated: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get the current total allocation
    pub fn current_allocation(&self) -> u64 {
        self.allocated.load(Ordering::Relaxed)
    }

    /// Check if an allocation of the given size would exceed limits
    ///
    /// Returns an error if:
    /// - The single allocation would exceed MAX_SINGLE_ALLOCATION
    /// - The cumulative allocation would exceed MAX_CUMULATIVE_ALLOCATION
    pub fn check_allocation(&self, size: u64) -> Result<(), AllocationError> {
        // Check single operation limit
        if size > MAX_SINGLE_ALLOCATION {
            return Err(AllocationError::SingleAllocationTooLarge {
                requested: size,
                max_allowed: MAX_SINGLE_ALLOCATION,
            });
        }

        // Check cumulative limit
        let current = self.current_allocation();
        if current.saturating_add(size) > MAX_CUMULATIVE_ALLOCATION {
            return Err(AllocationError::CumulativeAllocationExceeded {
                current,
                requested: size,
                max_allowed: MAX_CUMULATIVE_ALLOCATION,
            });
        }

        Ok(())
    }

    /// Record an allocation
    ///
    /// Should be called after a successful allocation to update the budget.
    /// Uses overflow protection to prevent wrap-around bypass of limits.
    pub fn record_allocation(&self, size: u64) {
        self.allocated
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                let new = current.saturating_add(size);
                // Prevent allocation if it would exceed the limit
                if new > MAX_CUMULATIVE_ALLOCATION {
                    None // Reject the update
                } else {
                    Some(new) // Accept the update
                }
            })
            .ok(); // Ignore error if allocation would exceed limit
    }

    /// Check and record an allocation in one operation
    ///
    /// Returns Ok if the allocation is allowed and records it.
    /// Returns Err if the allocation would exceed limits.
    pub fn allocate(&self, size: u64) -> Result<(), AllocationError> {
        self.check_allocation(size)?;
        self.record_allocation(size);
        Ok(())
    }

    /// Check if a vector allocation is allowed
    ///
    /// Convenience method for checking Vec allocations.
    pub fn check_vec_allocation<T>(&self, count: usize) -> Result<(), AllocationError> {
        let size = count.checked_mul(std::mem::size_of::<T>()).ok_or(
            AllocationError::SingleAllocationTooLarge {
                requested: u64::MAX,
                max_allowed: MAX_SINGLE_ALLOCATION,
            },
        )? as u64;
        self.check_allocation(size)
    }

    /// Record a vector allocation
    ///
    /// Convenience method for recording Vec allocations.
    pub fn record_vec_allocation<T>(&self, count: usize) {
        let size = count.saturating_mul(std::mem::size_of::<T>()) as u64;
        self.record_allocation(size);
    }

    /// Allocate and track a vector in one operation
    ///
    /// Returns Ok if the allocation is allowed and returns a Vec with the given capacity.
    /// Returns Err if the allocation would exceed limits.
    pub fn allocate_vec<T>(&self, capacity: usize) -> Result<Vec<T>, AllocationError> {
        self.check_vec_allocation::<T>(capacity)?;
        let size = capacity.saturating_mul(std::mem::size_of::<T>()) as u64;
        self.record_allocation(size);
        Ok(Vec::with_capacity(capacity))
    }

    /// Reset the budget (for testing or session reset)
    #[cfg(test)]
    pub fn reset(&self) {
        self.allocated.store(0, Ordering::Relaxed);
    }

    /// Get the maximum cumulative allocation limit
    pub const fn max_cumulative_allocation() -> u64 {
        MAX_CUMULATIVE_ALLOCATION
    }

    /// Get the maximum single allocation limit
    pub const fn max_single_allocation() -> u64 {
        MAX_SINGLE_ALLOCATION
    }
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self::new()
    }
}

/// Allocation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AllocationError {
    /// Single allocation exceeds the maximum allowed size
    SingleAllocationTooLarge { requested: u64, max_allowed: u64 },

    /// Cumulative allocation would exceed the maximum allowed
    CumulativeAllocationExceeded {
        current: u64,
        requested: u64,
        max_allowed: u64,
    },
}

impl std::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SingleAllocationTooLarge {
                requested,
                max_allowed,
            } => write!(
                f,
                "Allocation of {} bytes exceeds maximum of {} bytes",
                requested, max_allowed
            ),
            Self::CumulativeAllocationExceeded {
                current,
                requested,
                max_allowed,
            } => write!(
                f,
                "Cumulative allocation of {} bytes + {} bytes would exceed maximum of {} bytes",
                current, requested, max_allowed
            ),
        }
    }
}

impl std::error::Error for AllocationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_allocation_limit() {
        let budget = ResourceBudget::new();

        // Within limit
        assert!(budget.check_allocation(MAX_SINGLE_ALLOCATION).is_ok());

        // Exceeds limit
        assert!(budget.check_allocation(MAX_SINGLE_ALLOCATION + 1).is_err());
    }

    #[test]
    fn test_cumulative_allocation_limit() {
        let budget = ResourceBudget::new();

        // Multiple small allocations should be allowed
        for _ in 0..10 {
            assert!(budget.allocate(10 * 1024 * 1024).is_ok());
        }

        // Eventually should hit cumulative limit
        loop {
            match budget.allocate(10 * 1024 * 1024) {
                Ok(()) => continue,
                Err(AllocationError::CumulativeAllocationExceeded { .. }) => break,
                Err(e) => panic!("Unexpected error: {:?}", e),
            }
        }
    }

    #[test]
    fn test_vec_allocation() {
        let budget = ResourceBudget::new();

        // Small vec allocation
        assert!(budget.check_vec_allocation::<u8>(1000).is_ok());

        // Large vec allocation
        assert!(budget.allocate_vec::<u8>(10 * 1024 * 1024).is_ok());

        // Very large vec should fail
        assert!(budget.allocate_vec::<u8>(100 * 1024 * 1024).is_err());
    }

    #[test]
    fn test_reset() {
        let budget = ResourceBudget::new();

        // Allocate some memory
        budget.allocate(10 * 1024 * 1024).unwrap();
        assert!(budget.current_allocation() > 0);

        // Reset
        budget.reset();
        assert_eq!(budget.current_allocation(), 0);
    }

    #[test]
    fn test_shared_budget() {
        let budget1 = ResourceBudget::new();
        let budget2 = budget1.clone(); // Shares the same counter

        budget1.allocate(1024).unwrap();
        assert_eq!(budget2.current_allocation(), 1024);

        budget2.allocate(2048).unwrap();
        assert_eq!(budget1.current_allocation(), 3072);
    }
}
