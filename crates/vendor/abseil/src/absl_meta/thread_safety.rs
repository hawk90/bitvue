//! Thread safety traits for type checking.

/// Trait to detect if a type is Send (can be transferred across threads).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsSend;
///
/// assert!(IsSend::<i32>::VALUE);
/// assert!(IsSend::<String>::VALUE);
/// ```
pub trait IsSend {
    /// True if the type is Send.
    const VALUE: bool = false;
}

impl<T: ?Sized> IsSend for T
where
    T: Send,
{
    const VALUE: bool = true;
}

/// Trait to detect if a type is Sync (can be shared across threads).
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::IsSync;
///
/// assert!(IsSync::<i32>::VALUE);
/// assert!(IsSync::<&i32>::VALUE);
/// ```
pub trait IsSync {
    /// True if the type is Sync.
    const VALUE: bool = false;
}

impl<T: ?Sized> IsSync for T
where
    T: Sync,
{
    const VALUE: bool = true;
}

/// Trait to detect if a type implements Clone.
///
/// # Examples
///
/// ```rust
/// use abseil::absl_meta::HasCloneTrait;
///
/// assert!(HasCloneTrait::<i32>::VALUE);
/// assert!(HasCloneTrait::<Vec<i32>>::VALUE);
/// ```
pub trait HasCloneTrait {
    /// True if the type implements Clone.
    const VALUE: bool = false;
}

impl<T: Clone> HasCloneTrait for T {
    const VALUE: bool = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for IsSend
    #[test]
    fn test_is_send() {
        assert!(IsSend::<i32>::VALUE);
        assert!(IsSend::<String>::VALUE);
    }

    // Tests for IsSync
    #[test]
    fn test_is_sync() {
        assert!(IsSync::<i32>::VALUE);
        assert!(IsSync::<&i32>::VALUE);
    }

    // Tests for HasCloneTrait
    #[test]
    fn test_has_clone_trait() {
        assert!(HasCloneTrait::<i32>::VALUE);
        assert!(HasCloneTrait::<Vec<i32>>::VALUE);
    }
}
