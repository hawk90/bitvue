//! Function traits and utility functions

/// Trait for functions that can be applied to a value.
pub trait Apply<A> {
    type Output;
    fn apply(self, arg: A) -> Self::Output;
}

impl<A, B, F: FnOnce(A) -> B> Apply<A> for F {
    type Output = B;

    #[inline]
    fn apply(self, arg: A) -> Self::Output {
        self(arg)
    }
}

/// Trait for functions that can be called multiple times.
pub trait Callable<A, B> {
    fn call(&mut self, arg: A) -> B;
}

impl<A, B, F: FnMut(A) -> B> Callable<A, B> for F {
    #[inline]
    fn call(&mut self, arg: A) -> B {
        self(arg)
    }
}

/// Calls a function with each element of a slice, collecting results.
#[inline]
pub fn map_slice<'a, A, B, F>(slice: &'a [A], mut f: F) -> Vec<B>
where
    F: FnMut(&'a A) -> B,
{
    slice.iter().map(|x| f(x)).collect()
}

/// Calls a function with each element of a slice, in place.
#[inline]
pub fn for_each_slice<A, F>(slice: &mut [A], mut f: F)
where
    F: FnMut(&mut A),
{
    for x in slice {
        f(x);
    }
}

/// Finds an element in a slice using a predicate.
#[inline]
pub fn find_slice<'a, A, F>(slice: &'a [A], mut f: F) -> Option<&'a A>
where
    F: FnMut(&A) -> bool,
{
    slice.iter().find(|x| f(x))
}

/// Partitions a slice into two based on a predicate.
///
/// Returns (matching, non_matching).
#[inline]
pub fn partition_slice<'a, A, F>(slice: &'a [A], mut f: F) -> (Vec<&'a A>, Vec<&'a A>)
where
    F: FnMut(&A) -> bool,
{
    let mut matching = Vec::new();
    let mut non_matching = Vec::new();

    for x in slice {
        if f(x) {
            matching.push(x);
        } else {
            non_matching.push(x);
        }
    }

    (matching, non_matching)
}

/// Folds a slice using a function.
#[inline]
pub fn fold_slice<A, B, F>(slice: &[A], init: B, mut f: F) -> B
where
    F: FnMut(B, &A) -> B,
{
    let mut acc = init;
    for x in slice {
        acc = f(acc, x);
    }
    acc
}

/// Checks if all elements satisfy a predicate.
#[inline]
pub fn all_slice<A, F>(slice: &[A], mut f: F) -> bool
where
    F: FnMut(&A) -> bool,
{
    slice.iter().all(|x| f(x))
}

/// Checks if any element satisfies a predicate.
#[inline]
pub fn any_slice<A, F>(slice: &[A], mut f: F) -> bool
where
    F: FnMut(&A) -> bool,
{
    slice.iter().any(|x| f(x))
}

/// Counts elements satisfying a predicate.
#[inline]
pub fn count_slice<A, F>(slice: &[A], mut f: F) -> usize
where
    F: FnMut(&A) -> bool,
{
    slice.iter().filter(|x| f(x)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply() {
        let f = |x: i32| x * 2;
        assert_eq!(f.apply(5), 10);
    }

    #[test]
    fn test_callable() {
        let mut f = |x: i32| x * 2;
        assert_eq!(f.call(5), 10);
    }

    #[test]
    fn test_map_slice() {
        let slice = [1, 2, 3, 4, 5];
        let result = map_slice(&slice, |&x| x * 2);
        assert_eq!(result, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_find_slice() {
        let slice = [1, 2, 3, 4, 5];
        assert_eq!(find_slice(&slice, |&x| x == 3), Some(&3));
        assert_eq!(find_slice(&slice, |&x| x == 10), None);
    }

    #[test]
    fn test_partition_slice() {
        let slice = [1, 2, 3, 4, 5, 6];
        let (evens, odds) = partition_slice(&slice, |&x| x % 2 == 0);
        assert_eq!(evens, vec![&2, &4, &6]);
        assert_eq!(odds, vec![&1, &3, &5]);
    }

    #[test]
    fn test_fold_slice() {
        let slice = [1, 2, 3, 4, 5];
        let sum = fold_slice(&slice, 0, |acc, &x| acc + x);
        assert_eq!(sum, 15);
    }

    #[test]
    fn test_all_slice() {
        let slice = [2, 4, 6, 8];
        assert!(all_slice(&slice, |&x| x % 2 == 0));
        assert!(!all_slice(&slice, |&x| x > 4));
    }

    #[test]
    fn test_any_slice() {
        let slice = [1, 2, 3, 4, 5];
        assert!(any_slice(&slice, |&x| x == 3));
        assert!(!any_slice(&slice, |&x| x == 10));
    }

    #[test]
    fn test_count_slice() {
        let slice = [1, 2, 3, 4, 5, 6];
        assert_eq!(count_slice(&slice, |&x| x % 2 == 0), 3);
    }
}
