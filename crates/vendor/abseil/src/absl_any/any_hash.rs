//! AnyHash - Hashing for type-erased values.

use core::any::TypeId;
use crate::absl_any::any_box::AnyBox;

/// Trait for hashing type-erased values.
pub trait AnyHash {
    /// Returns a hash of the type-erased value.
    fn any_hash(&self) -> u64;
}

impl<T: core::hash::Hash + 'static> AnyHash for T {
    fn any_hash(&self) -> u64 {
        use core::hash::Hasher;
        // Simple FNV hash
        let mut hash = 0xcbf29ce484222325u64;
        hash = hash.wrapping_mul(0x100000001b3);
        let type_id_hash = TypeId::of::<T>().hash();
        hash ^= type_id_hash;
        hash
    }
}

/// Computes a hash value for a type-erased value.
///
/// # Examples
///
/// ```
/// use abseil::absl_any::{any_hash, AnyBox};
///
/// let boxed = AnyBox::new(42i32);
/// let hash = any_hash(&boxed);
/// assert!(hash > 0);
/// ```
pub fn any_hash(value: &AnyBox) -> u64 {
    let mut hasher = FnvHasher::default();
    hasher.update_type_id(value.type_id());
    if let Some(&v) = value.downcast_ref::<i32>() {
        hasher.update_i32(v);
    } else if let Some(&v) = value.downcast_ref::<i64>() {
        hasher.update_i64(v);
    } else if let Some(v) = value.downcast_ref::<alloc::string::String>() {
        hasher.update_bytes(v.as_bytes());
    } else if let Some(&v) = value.downcast_ref::<&str>() {
        hasher.update_bytes(v.as_bytes());
    } else {
        hasher.update_bytes(value.type_name().as_bytes());
    }
    hasher.finish()
}

/// Simple FNV hasher for Any values.
struct FnvHasher {
    state: u64,
}

impl Default for FnvHasher {
    fn default() -> Self {
        Self {
            state: 0xcbf29ce484222325,
        }
    }
}

impl FnvHasher {
    fn update(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= byte as u64;
            self.state = self.state.wrapping_mul(0x100000001b3);
        }
    }

    fn update_type_id(&mut self, type_id: TypeId) {
        self.update(&type_id.hash().to_le_bytes());
    }

    fn update_i32(&mut self, value: i32) {
        self.update(&value.to_le_bytes());
    }

    fn update_i64(&mut self, value: i64) {
        self.update(&value.to_le_bytes());
    }

    fn update_bytes(&mut self, bytes: &[u8]) {
        self.update(bytes);
    }

    fn finish(self) -> u64 {
        self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_any_hash_i32() {
        let a = AnyBox::new(42i32);
        let b = AnyBox::new(42i32);
        assert_eq!(any_hash(&a), any_hash(&b));
    }

    #[test]
    fn test_any_hash_different() {
        let a = AnyBox::new(42i32);
        let b = AnyBox::new(100i32);
        assert_ne!(any_hash(&a), any_hash(&b));
    }
}
