#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
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
