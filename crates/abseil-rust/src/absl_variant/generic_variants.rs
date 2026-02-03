//! Generic Variant types - Variant2 through Variant8 for type-safe unions.

/// A variant that can hold one of two types.
#[derive(Clone, Debug, PartialEq)]
pub enum Variant2<A, B> {
    First(A),
    Second(B),
}

impl<A, B> Variant2<A, B> {
    /// Returns true if this is the First variant.
    pub fn is_first(&self) -> bool {
        matches!(self, Variant2::First(_))
    }

    /// Returns true if this is the Second variant.
    pub fn is_second(&self) -> bool {
        matches!(self, Variant2::Second(_))
    }

    /// Returns the First value if present.
    pub fn as_first(&self) -> Option<&A> {
        match self {
            Variant2::First(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the Second value if present.
    pub fn as_second(&self) -> Option<&B> {
        match self {
            Variant2::Second(v) => Some(v),
            _ => None,
        }
    }

    /// Maps the First value.
    pub fn map_first<F, A2>(self, f: F) -> Variant2<A2, B>
    where
        F: FnOnce(A) -> A2,
    {
        match self {
            Variant2::First(v) => Variant2::First(f(v)),
            Variant2::Second(v) => Variant2::Second(v),
        }
    }

    /// Maps the Second value.
    pub fn map_second<F, B2>(self, f: F) -> Variant2<A, B2>
    where
        F: FnOnce(B) -> B2,
    {
        match self {
            Variant2::First(v) => Variant2::First(v),
            Variant2::Second(v) => Variant2::Second(f(v)),
        }
    }

    /// Maps either value to a common type.
    pub fn map_either<F, R>(self, f: F) -> R
    where
        F: FnOnce(Variant2<A, B>) -> R,
    {
        f(self)
    }

    /// Converts to Result, treating First as Ok and Second as Err.
    pub fn to_result(self) -> Result<A, B> {
        match self {
            Variant2::First(v) => Ok(v),
            Variant2::Second(v) => Err(v),
        }
    }

    /// Converts from Result.
    pub fn from_result(r: Result<A, B>) -> Self {
        match r {
            Ok(v) => Variant2::First(v),
            Err(v) => Variant2::Second(v),
        }
    }

    /// Swaps the variants.
    pub fn swap(self) -> Variant2<B, A> {
        match self {
            Variant2::First(v) => Variant2::Second(v),
            Variant2::Second(v) => Variant2::First(v),
        }
    }
}

impl<A: Default, B> Default for Variant2<A, B> {
    fn default() -> Self {
        Variant2::First(A::default())
    }
}

/// A variant that can hold one of three types.
#[derive(Clone, Debug, PartialEq)]
pub enum Variant3<A, B, C> {
    First(A),
    Second(B),
    Third(C),
}

impl<A, B, C> Variant3<A, B, C> {
    /// Returns the index of the active variant (0, 1, or 2).
    pub fn index(&self) -> usize {
        match self {
            Variant3::First(_) => 0,
            Variant3::Second(_) => 1,
            Variant3::Third(_) => 2,
        }
    }

    /// Returns true if the variant at the given index is active.
    pub fn is_at(&self, index: usize) -> bool {
        self.index() == index
    }
}

impl<A: Default, B, C> Default for Variant3<A, B, C> {
    fn default() -> Self {
        Variant3::First(A::default())
    }
}

/// A variant that can hold one of four types.
#[derive(Clone, Debug, PartialEq)]
pub enum Variant4<A, B, C, D> {
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
}

impl<A, B, C, D> Variant4<A, B, C, D> {
    /// Returns the index of the active variant (0-3).
    pub fn index(&self) -> usize {
        match self {
            Variant4::First(_) => 0,
            Variant4::Second(_) => 1,
            Variant4::Third(_) => 2,
            Variant4::Fourth(_) => 3,
        }
    }

    /// Returns true if the variant at the given index is active.
    pub fn is_at(&self, index: usize) -> bool {
        self.index() == index
    }
}

impl<A: Default, B, C, D> Default for Variant4<A, B, C, D> {
    fn default() -> Self {
        Variant4::First(A::default())
    }
}

/// A variant that can hold one of five types.
#[derive(Clone, Debug, PartialEq)]
pub enum Variant5<A, B, C, D, E> {
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
    Fifth(E),
}

impl<A, B, C, D, E> Variant5<A, B, C, D, E> {
    /// Returns the index of the active variant (0-4).
    pub fn index(&self) -> usize {
        match self {
            Variant5::First(_) => 0,
            Variant5::Second(_) => 1,
            Variant5::Third(_) => 2,
            Variant5::Fourth(_) => 3,
            Variant5::Fifth(_) => 4,
        }
    }

    /// Returns true if the variant at the given index is active.
    pub fn is_at(&self, index: usize) -> bool {
        self.index() == index
    }
}

impl<A: Default, B, C, D, E> Default for Variant5<A, B, C, D, E> {
    fn default() -> Self {
        Variant5::First(A::default())
    }
}

/// A variant that can hold one of six types.
#[derive(Clone, Debug, PartialEq)]
pub enum Variant6<A, B, C, D, E, F> {
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
    Fifth(E),
    Sixth(F),
}

impl<A, B, C, D, E, F> Variant6<A, B, C, D, E, F> {
    /// Returns the index of the active variant (0-5).
    pub fn index(&self) -> usize {
        match self {
            Variant6::First(_) => 0,
            Variant6::Second(_) => 1,
            Variant6::Third(_) => 2,
            Variant6::Fourth(_) => 3,
            Variant6::Fifth(_) => 4,
            Variant6::Sixth(_) => 5,
        }
    }

    /// Returns true if the variant at the given index is active.
    pub fn is_at(&self, index: usize) -> bool {
        self.index() == index
    }
}

impl<A: Default, B, C, D, E, F> Default for Variant6<A, B, C, D, E, F> {
    fn default() -> Self {
        Variant6::First(A::default())
    }
}

/// A variant that can hold one of seven types.
#[derive(Clone, Debug, PartialEq)]
pub enum Variant7<A, B, C, D, E, F, G> {
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
    Fifth(E),
    Sixth(F),
    Seventh(G),
}

impl<A, B, C, D, E, F, G> Variant7<A, B, C, D, E, F, G> {
    /// Returns the index of the active variant (0-6).
    pub fn index(&self) -> usize {
        match self {
            Variant7::First(_) => 0,
            Variant7::Second(_) => 1,
            Variant7::Third(_) => 2,
            Variant7::Fourth(_) => 3,
            Variant7::Fifth(_) => 4,
            Variant7::Sixth(_) => 5,
            Variant7::Seventh(_) => 6,
        }
    }

    /// Returns true if the variant at the given index is active.
    pub fn is_at(&self, index: usize) -> bool {
        self.index() == index
    }
}

impl<A: Default, B, C, D, E, F, G> Default for Variant7<A, B, C, D, E, F, G> {
    fn default() -> Self {
        Variant7::First(A::default())
    }
}

/// A variant that can hold one of eight types.
#[derive(Clone, Debug, PartialEq)]
pub enum Variant8<A, B, C, D, E, F, G, H> {
    First(A),
    Second(B),
    Third(C),
    Fourth(D),
    Fifth(E),
    Sixth(F),
    Seventh(G),
    Eighth(H),
}

impl<A, B, C, D, E, F, G, H> Variant8<A, B, C, D, E, F, G, H> {
    /// Returns the index of the active variant (0-7).
    pub fn index(&self) -> usize {
        match self {
            Variant8::First(_) => 0,
            Variant8::Second(_) => 1,
            Variant8::Third(_) => 2,
            Variant8::Fourth(_) => 3,
            Variant8::Fifth(_) => 4,
            Variant8::Sixth(_) => 5,
            Variant8::Seventh(_) => 6,
            Variant8::Eighth(_) => 7,
        }
    }

    /// Returns true if the variant at the given index is active.
    pub fn is_at(&self, index: usize) -> bool {
        self.index() == index
    }
}

impl<A: Default, B, C, D, E, F, G, H> Default for Variant8<A, B, C, D, E, F, G, H> {
    fn default() -> Self {
        Variant8::First(A::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Variant2 tests
    #[test]
    fn test_variant2_first() {
        let v: Variant2<i32, String> = Variant2::First(42);
        assert!(v.is_first());
        assert!(!v.is_second());
        assert_eq!(v.as_first(), Some(&42));
        assert_eq!(v.as_second(), None);
    }

    #[test]
    fn test_variant2_second() {
        let v: Variant2<i32, String> = Variant2::Second("hello".to_string());
        assert!(!v.is_first());
        assert!(v.is_second());
        assert_eq!(v.as_first(), None);
        assert_eq!(v.as_second(), Some(&"hello".to_string()));
    }

    #[test]
    fn test_variant2_map_first() {
        let v: Variant2<i32, String> = Variant2::First(42);
        let mapped = v.map_first(|i| i * 2);
        assert_eq!(mapped.as_first(), Some(&84));
    }

    #[test]
    fn test_variant2_map_second() {
        let v: Variant2<i32, String> = Variant2::Second("hello".to_string());
        let mapped = v.map_second(|s| s.to_uppercase());
        assert_eq!(mapped.as_second(), Some(&"HELLO".to_string()));
    }

    #[test]
    fn test_variant2_to_result() {
        let v: Variant2<i32, String> = Variant2::First(42);
        assert_eq!(v.to_result(), Ok(42));

        let v: Variant2<i32, String> = Variant2::Second("error".to_string());
        assert_eq!(v.to_result(), Err("error".to_string()));
    }

    #[test]
    fn test_variant2_from_result() {
        let v: Variant2<i32, String> = Variant2::from_result(Ok(42));
        assert!(v.is_first());

        let v: Variant2<i32, String> = Variant2::from_result(Err("error".to_string()));
        assert!(v.is_second());
    }

    #[test]
    fn test_variant2_swap() {
        let v: Variant2<i32, String> = Variant2::First(42);
        let swapped: Variant2<String, i32> = v.swap();
        assert_eq!(swapped.as_second(), Some(&42));
    }

    #[test]
    fn test_variant2_default() {
        let v: Variant2<i32, String> = Variant2::default();
        assert!(v.is_first());
        assert_eq!(v.as_first(), Some(&0));
    }

    // Variant3-8 tests
    #[test]
    fn test_variant3_index() {
        let v: Variant3<i32, String, bool> = Variant3::First(42);
        assert_eq!(v.index(), 0);

        let v: Variant3<i32, String, bool> = Variant3::Second("hello".to_string());
        assert_eq!(v.index(), 1);

        let v: Variant3<i32, String, bool> = Variant3::Third(true);
        assert_eq!(v.index(), 2);
    }

    #[test]
    fn test_variant3_is_at() {
        let v: Variant3<i32, String, bool> = Variant3::First(42);
        assert!(v.is_at(0));
        assert!(!v.is_at(1));
        assert!(!v.is_at(2));
    }

    #[test]
    fn test_variant4_index() {
        let v: Variant4<i32, String, bool, f64> = Variant4::Fourth(3.14);
        assert_eq!(v.index(), 3);
    }

    #[test]
    fn test_variant5_index() {
        let v: Variant5<i32, String, bool, f64, u64> = Variant5::Fifth(42);
        assert_eq!(v.index(), 4);
    }

    #[test]
    fn test_variant6_index() {
        let v: Variant6<i32, String, bool, f64, u64, i64> = Variant6::Sixth(42);
        assert_eq!(v.index(), 5);
    }

    #[test]
    fn test_variant7_index() {
        let v: Variant7<i32, String, bool, f64, u64, i64, u32> = Variant7::Seventh(42);
        assert_eq!(v.index(), 6);
    }

    #[test]
    fn test_variant8_index() {
        let v: Variant8<i32, String, bool, f64, u64, i64, u32, u8> = Variant8::Eighth(42);
        assert_eq!(v.index(), 7);
    }
}
