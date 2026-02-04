//! Command-line flag utilities.
//!
//! This module provides flag utilities similar to Abseil's `absl/flags` directory.
//! It provides a simple but flexible command-line flag parsing system.
//!
//! # Overview
//!
//! The flag parsing system allows you to define and parse command-line arguments
//! with type safety and default values. It supports:
//!
//! - Boolean flags ([`BoolFlag`])
//! - String flags ([`StringFlag`])
//! - Integer flags ([`IntFlag`])
//! - Automatic help generation
//! - Flag validation
//!
//! # Modules
//!
//! - [`flags`] - Flag parsing and common flags
//!
//! # Examples
//!
//! ```rust,ignore
//! use abseil::absl_flags::flags::*;
//!
//! // Define flags
//! static VERBOSE: BoolFlag = BoolFlag::new("verbose", false, "Enable verbose output");
//! static OUTPUT: StringFlag = StringFlag::new("output", "", "Output file path");
//! static COUNT: IntFlag = IntFlag::new("count", 0, "Number of iterations");
//!
//! // Parse command line
//! let args: Vec<String> = env::args().collect();
//! let remaining = flag::parse(&args);
//!
//! // Access flag values
//! if VERBOSE.value() {
//!     println!("Verbose mode enabled");
//! }
//! ```
//!
//! # Standard Flags
//!
//! The following standard flags are automatically available:
//!
//! - `USAGE` - Prints usage information and exits
//! - `VERBOSE` - Enables verbose logging
//! - `flag` - help flag (alias for --help)
//!
//! # Flag Syntax
//!
//! Flags can be specified in the following formats:
//!
//! - `--flag=value` - Long form with equals
//! - `--flag value` - Long form with space
//! - `-f value` - Short form with space
//! - `-fvalue` - Short form without space
//! - `--flag` - Boolean flags (true)
//! - `--noflag` - Boolean flags (false)

pub mod flags;

// Error types
pub mod error;

// Flag parsing functions
pub mod parse;

// Flag validation utilities
pub mod validation;

// Argument utilities
pub mod args;

// Parse configuration
pub mod config;

// Help formatting
pub mod help;

// Flag value wrapper
pub mod flag_value;

// Flag constraints
pub mod constraints;

// Flag definition
pub mod definition;

// Flag builder
pub mod builder;

// Subcommand and command parser
pub mod subcommand;

// Parse result
pub mod parse_result;

// Re-exports from flags module
pub use flags::{
    BoolFlag, Flag, FlagParseError, FlagResult, FlagType, IntFlag, StringFlag,
    USAGE, VERBOSE, flag,
};

// Re-exports from error module
pub use error::{error_from_parse, FlagsError};

// Re-exports from parse module
pub use parse::{has_help_flag, parse_flags, program_name, usage_string};

// Re-exports from validation module
pub use validation::{validate_file_extension, validate_flag_range, validate_non_empty};

// Re-exports from args module
pub use args::{
    is_flag, is_long_flag, is_short_flag, parse_flag_name, parse_flag_value, split_args,
};

// Re-exports from config module
pub use config::{parse_flags_with_config, ParseConfig};

// Re-exports from help module
pub use help::{default_banner, format_flag_help};

// Re-exports from flag_value module
pub use flag_value::FlagValue;

// Re-exports from constraints module
pub use constraints::{ChoiceConstraint, FlagConstraint, RangeConstraint};

// Re-exports from definition module
pub use definition::FlagDefinition;

// Re-exports from builder module
pub use builder::FlagBuilder;

// Re-exports from subcommand module
pub use subcommand::{CommandParser, Subcommand};

// Re-exports from parse_result module
pub use parse_result::{get_env_fallback, parse_with_subcommands, FlagAliases, ParseResult};
