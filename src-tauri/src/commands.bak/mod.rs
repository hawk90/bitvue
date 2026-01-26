//! Tauri command handlers (API Layer)
//!
//! This module contains all Tauri commands that can be invoked from the frontend.
//! Each command is a Rust function with the #[tauri::command] attribute.
//!
//! Layer separation:
//! - Commands: API boundary, parameter validation, response formatting
//! - Services: Business logic, orchestration
//! - Core: Low-level operations, state management

pub mod file_commands;
pub mod frame_commands;
pub mod overlays;
pub mod selection_commands;
pub mod tooltip_commands;
pub mod tooltip_helpers;

// Re-export commands for convenience
pub use file_commands::*;
pub use frame_commands::*;
pub use selection_commands::*;
pub use tooltip_commands::*;

use bitvue_core::Core;
use std::sync::{Arc, Mutex};

use crate::services::DecodeService;

/// Shared application state
pub struct AppState {
    pub core: Arc<Mutex<Core>>,
    pub decode_service: Arc<Mutex<DecodeService>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            core: Arc::new(Mutex::new(Core::new())),
            decode_service: Arc::new(Mutex::new(DecodeService::new())),
        }
    }
}
