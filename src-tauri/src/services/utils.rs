//! Shared utility macros and functions for services

/// Helper macro to safely lock mutexes with proper error handling
///
/// Prevents panic on mutex poisoning by returning an error instead of
/// unwrapping and potentially crashing the application.
///
/// # Example
/// ```rust
/// use crate::services::utils::lock_mutex;
///
/// let data = lock_mutex!(some_mutex);
/// // Returns error if mutex is poisoned instead of panicking
/// ```
#[macro_export]
macro_rules! lock_mutex {
    ($mutex:expr) => {
        $mutex.lock().map_err(|e| format!("Mutex poisoned: {}", e))?
    };
}

/// Helper macro to safely lock RwLocks for reading with proper error handling
///
/// # Example
/// ```rust
/// use crate::services::utils::lock_read;
///
/// let data = lock_read!(some_rwlock);
/// // Returns error if RwLock is poisoned instead of panicking
/// ```
#[macro_export]
macro_rules! lock_read {
    ($rwlock:expr) => {
        $rwlock.read().map_err(|e| format!("RwLock poisoned: {}", e))?
    };
}

/// Helper macro to safely lock RwLocks for writing with proper error handling
///
/// # Example
/// ```rust
/// use crate::services::utils::lock_write;
///
/// let mut data = lock_write!(some_rwlock);
/// // Returns error if RwLock is poisoned instead of panicking
/// ```
#[macro_export]
macro_rules! lock_write {
    ($rwlock:expr) => {
        $rwlock.write().map_err(|e| format!("RwLock poisoned: {}", e))?
    };
}

// Re-export macros for use in other modules
pub use lock_mutex;
pub use lock_read;
pub use lock_write;
