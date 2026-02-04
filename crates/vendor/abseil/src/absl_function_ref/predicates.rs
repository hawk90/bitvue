//! Predicate utilities - eq, ne, lt, gt, in_range

/// Creates a predicate that checks equality with a value.
#[inline]
pub fn eq<T: PartialEq>(value: T) -> impl Fn(&T) -> bool {
    move |other| *other == value
}

/// Creates a predicate that checks inequality with a value.
#[inline]
pub fn ne<T: PartialEq>(value: T) -> impl Fn(&T) -> bool {
    move |other| *other != value
}

/// Creates a predicate that checks if a value is less than another.
#[inline]
pub fn lt<T: PartialOrd>(value: T) -> impl Fn(&T) -> bool
where
    T: Clone,
{
    let value = value.clone();
    move |other| other.partial_cmp(&value) == Some(core::cmp::Ordering::Less)
}

/// Creates a predicate that checks if a value is greater than another.
#[inline]
pub fn gt<T: PartialOrd>(value: T) -> impl Fn(&T) -> bool
where
    T: Clone,
{
    let value = value.clone();
    move |other| other.partial_cmp(&value) == Some(core::cmp::Ordering::Greater)
}

/// Creates a predicate that checks if a value is in a range.
#[inline]
pub fn in_range<T: PartialOrd>(min: T, max: T) -> impl Fn(&T) -> bool
where
    T: Clone,
{
    let min = min.clone();
    let max = max.clone();
    move |value| {
        value.partial_cmp(&min) != Some(core::cmp::Ordering::Less)
            && value.partial_cmp(&max) != Some(core::cmp::Ordering::Greater)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq() {
        let predicate = eq(5);
        assert!(predicate(&5));
        assert!(!predicate(&10));
    }

    #[test]
    fn test_ne() {
        let predicate = ne(5);
        assert!(!predicate(&5));
        assert!(predicate(&10));
    }

    #[test]
    fn test_lt() {
        let predicate = lt(10);
        assert!(predicate(&5));
        assert!(!predicate(&15));
    }

    #[test]
    fn test_gt() {
        let predicate = gt(5);
        assert!(predicate(&10));
        assert!(!predicate(&3));
    }

    #[test]
    fn test_in_range() {
        let predicate = in_range(5, 10);
        assert!(predicate(&7));
        assert!(predicate(&5));
        assert!(predicate(&10));
        assert!(!predicate(&4));
        assert!(!predicate(&11));
    }
}
