//! Stack - A stack container adapter (LIFO)

use alloc::vec::Vec;

/// A stack container adapter (LIFO).
///
/// Provides stack operations on top of a vector.
#[derive(Clone, Debug)]
pub struct Stack<T> {
    data: Vec<T>,
}

impl<T> Stack<T> {
    /// Creates a new empty stack.
    #[inline]
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Creates a stack with pre-allocated capacity.
    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Pushes a value onto the stack.
    #[inline]
    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    /// Pops a value from the stack.
    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    /// Peeks at the top value without removing it.
    #[inline]
    pub fn peek(&self) -> Option<&T> {
        self.data.last()
    }

    /// Returns the number of elements on the stack.
    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the stack is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Clears the stack.
    #[inline]
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl<T: Default> Default for Stack<T> {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stack() {
        let mut stack = Stack::new();
        assert!(stack.is_empty());

        stack.push(1);
        stack.push(2);
        assert_eq!(stack.len(), 2);
        assert_eq!(stack.peek(), Some(&2));

        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_stack_with_capacity() {
        let stack: Stack<i32> = Stack::with_capacity(10);
        assert!(stack.is_empty());
    }
}
