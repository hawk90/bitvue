#![allow(dead_code)]
// Test private functions by exposing them with #[cfg(test)]
// This module tests internal functions that are not normally accessible

#[cfg(test)]
mod private_tests {
    // Import calculate_poc and other private functions from lib.rs
    // We'll need to add test-only exports to lib.rs

    // Commented out due to compilation issues
    // use bitvue_avc::{Sps, PictureType};

    // For now, test what we can access through public API
    // This file will be expanded once we add test-only exports to lib.rs
}
