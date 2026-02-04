//! Type-specific hash functions for Option, Result, and BTreeSet.

use alloc::collections::BTreeSet;
use core::hash::Hash;

use super::hash::hash_of;
use super::hash::HashState;
use super::symmetric::SymmetricHash;

/// Computes a hash code for an optional value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::type_hash::hash_option;
///
/// let some_hash = hash_option(&Some(42));
/// let none_hash = hash_option(&None::<i32>);
/// assert_ne!(some_hash, none_hash);
/// ```
pub fn hash_option<T: Hash>(value: &Option<T>) -> u64 {
    match value {
        Some(v) => hash_of(v),
        None => 0,
    }
}

/// Computes a hash code for a result value.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::type_hash::hash_result;
///
/// let ok_hash = hash_result(&Ok::<i32, i32>(42));
/// let err_hash = hash_result(&Err::<i32, i32>(42));
/// assert_ne!(ok_hash, err_hash);
/// ```
pub fn hash_result<T: Hash, E: Hash>(value: &Result<T, E>) -> u64 {
    match value {
        Ok(v) => hash_of(&(0u8, v)),
        Err(e) => hash_of(&(1u8, e)),
    }
}

/// Computes a hash for a BTreeSet (order-independent).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::type_hash::hash_btree_set;
/// use std::collections::BTreeSet;
///
/// let set: BTreeSet<i32> = [1, 2, 3].iter().copied().collect();
/// let hash = hash_btree_set(&set);
/// ```
pub fn hash_btree_set<T: Hash>(set: &BTreeSet<T>) -> u64 {
    let mut state = HashState::default();
    for item in set {
        state = state.update(item);
    }
    state.finalize()
}

/// Computes a hash for a BTreeSet using symmetric hashing.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_hash::type_hash::hash_btree_set_symmetric;
/// use std::collections::BTreeSet;
///
/// let set: BTreeSet<i32> = [1, 2, 3].iter().copied().collect();
/// let hash = hash_btree_set_symmetric(&set);
/// ```
pub fn hash_btree_set_symmetric<T: Hash>(set: &BTreeSet<T>) -> u64 {
    SymmetricHash::of_iter(set.iter().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_option() {
        let some_hash = hash_option(&Some(42));
        let none_hash = hash_option(&None::<i32>);
        assert_ne!(some_hash, none_hash);
    }

    #[test]
    fn test_hash_result() {
        let ok_hash = hash_result(&Ok::<i32, i32>(42));
        let err_hash = hash_result(&Err::<i32, i32>(42));
        assert_ne!(ok_hash, err_hash);
    }

    #[test]
    fn test_hash_btree_set() {
        let set: BTreeSet<i32> = [1, 2, 3].iter().copied().collect();
        let hash = hash_btree_set(&set);
        assert_ne!(hash, 0);
    }

    #[test]
    fn test_hash_btree_set_symmetric() {
        let set: BTreeSet<i32> = [1, 2, 3].iter().copied().collect();
        let hash = hash_btree_set_symmetric(&set);
        assert_ne!(hash, 0);
    }
}
