//! bitvue-log - Abseil-style logging system for Rust.
//!
//! This crate provides logging utilities inspired by Google's Abseil library,
//! integrated with the `tracing` ecosystem.
//!
//! # Features
//!
//! - **CHECK macros**: Informative assertion macros that print values on failure
//! - **VLOG system**: Runtime-controllable verbose logging levels
//! - **Rate-limited logging**: Control log output frequency in hot paths
//!
//! # Quick Start
//!
//! ```rust
//! use bitvue_log::{check, check_eq, vlog, log_every_n};
//!
//! fn process_data(data: &[u8]) {
//!     // CHECK macros - panic with informative messages
//!     check!(!data.is_empty(), "Data cannot be empty");
//!     check_eq!(data[0], 0x00, "Invalid header byte");
//!
//!     // VLOG - runtime-controllable verbose logging
//!     vlog!(1, "Processing {} bytes", data.len());
//!     vlog!(2, "First byte: {:02x}", data[0]);
//!
//!     for (i, byte) in data.iter().enumerate() {
//!         // Rate-limited logging - prevent spam in loops
//!         log_every_n!(trace, 1000, "Processed {} bytes", i);
//!     }
//! }
//! ```
//!
//! # Environment Variables
//!
//! - `VLOG_LEVEL=N`: Set global verbose logging level (0 = disabled)
//! - `VLOG_MODULE=mod1=N,mod2=M`: Set per-module VLOG levels
//!
//! # Initialization
//!
//! Call `init_from_env()` early in your application to read environment settings:
//!
//! ```rust
//! fn main() {
//!     // Initialize tracing first
//!     tracing_subscriber::fmt::init();
//!
//!     // Then initialize bitvue-log from environment
//!     bitvue_log::init_from_env();
//! }
//! ```

// Modules
pub mod check;
pub mod config;
pub mod rate_limit;
pub mod vlog;

// Re-export config initialization
pub use config::init_from_env;
pub use config::LogConfig;

// Re-export helper functions (needed for macro expansion)
#[doc(hidden)]
pub use check::__check_binary_failed;
#[doc(hidden)]
pub use check::__check_failed;
#[doc(hidden)]
pub use rate_limit::now_millis;
#[doc(hidden)]
pub use rate_limit::RateLimitState;

// Prelude module for convenient imports
pub mod prelude {
    //! Convenient re-exports for common usage.
    //!
    //! ```rust
    //! use bitvue_log::prelude::*;
    //! ```

    // CHECK macros
    pub use crate::check;
    pub use crate::check_eq;
    pub use crate::check_ge;
    pub use crate::check_gt;
    pub use crate::check_le;
    pub use crate::check_lt;
    pub use crate::check_ne;
    pub use crate::check_ok;
    pub use crate::check_some;
    pub use crate::check_streq;

    // Debug-only CHECK macros
    pub use crate::dcheck;
    pub use crate::dcheck_eq;
    pub use crate::dcheck_ge;
    pub use crate::dcheck_gt;
    pub use crate::dcheck_le;
    pub use crate::dcheck_lt;
    pub use crate::dcheck_ne;

    // VLOG macros
    pub use crate::dvlog;
    pub use crate::vlog;
    pub use crate::vlog_is_on;

    // Rate-limited logging
    pub use crate::dfatal;
    pub use crate::log_every_n;
    pub use crate::log_every_n_sec;
    pub use crate::log_first_n;
    pub use crate::log_if;
    pub use crate::log_if_every_n;
    pub use crate::plog;

    // Config
    pub use crate::init_from_env;
    pub use crate::LogConfig;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prelude_imports() {
        // Verify all prelude items are accessible
        use prelude::*;

        // CHECK macros compile
        check!(true);
        check_eq!(1, 1);
        check_ne!(1, 2);
        check_lt!(1, 2);
        check_le!(1, 1);
        check_gt!(2, 1);
        check_ge!(1, 1);

        // VLOG compiles (won't log since level is 0)
        vlog!(1, "test");

        // Rate-limited logging compiles
        log_every_n!(trace, 100, "test");
        log_first_n!(trace, 1, "test");
        log_if!(trace, true, "test");
    }

    #[test]
    fn test_config_accessible() {
        let config = LogConfig::global();
        config.set_vlog_level(0);
        assert_eq!(config.get_global_vlog_level(), 0);
    }
}
