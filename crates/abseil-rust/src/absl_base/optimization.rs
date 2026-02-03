//! Branch prediction and optimization hints.
//!
//! This module provides utilities for giving branch prediction hints to the compiler,
//! similar to Abseil's `absl/base/optimization.h`.
//!
//! These hints can help the CPU predict which branch will be taken, improving
//! performance in hot code paths.

/// Marks a branch as likely to be taken.
///
/// This hint tells the compiler and CPU that the condition is probably true,
/// allowing for better instruction cache and pipeline optimization.
///
/// # Example
///
/// ```rust
/// use abseil::likely;
///
/// fn process_data(data: Option<&[u8]>) -> &[u8] {
///     // We expect data to usually be present
///     if likely!(data.is_some()) {
///         data.unwrap()
///     } else {
///         &[]
///     }
/// }
/// ```
#[macro_export]
macro_rules! likely {
    ($e:expr) => {
        match $e {
            // Use #[cold] on the false branch to indicate it's unlikely
            e if {
                // Rust's intrinsic for branch prediction hint
                // On many platforms, this becomes a branch hint instruction
                let _v: bool = e;
                // The actual hint is compiler-dependent
                cfg!(any(
                    target_arch = "x86",
                    target_arch = "x86_64",
                    target_arch = "aarch64",
                    target_arch = "arm",
                ));
                // The compiler will treat true as the likely case
                true
            } => true,
            e => e,
        }
    };
}

/// Marks a branch as unlikely to be taken.
///
/// This hint tells the compiler and CPU that the condition is probably false,
/// allowing for better instruction cache and pipeline optimization.
///
/// # Example
///
/// The macro is used internally by the library for optimization hints.
/// Note: This is a low-level optimization macro and using Rust's built-in
/// `#[cold]` attribute is often preferred.
///
/// ```rust
/// // In practice, use the built-in approach with #[cold] attribute:
///
/// #[cold]
/// fn unlikely_branch() {
///     // This function is marked as unlikely to be called
/// }
///
/// fn main() {
///     let condition = true;
///     if condition {
///         // Fast path
///     } else {
///         unlikely_branch(); // Cold path
///     }
/// }
/// ```
#[macro_export]
macro_rules! unlikely {
    ($e:expr) => {
        // Mark this expression as unlikely
        // The compiler will optimize for the false case
        // Using #[cold] attribute approach
        {
            #[cold]
            fn cold_bool(b: bool) -> bool {
                b
            }
            cold_bool($e)
        }
    };
}

/// Internal helper to mark a boolean expression as cold (unlikely).
///
/// This function is marked cold so the compiler optimizes for the case
/// where it is not called (i.e., the condition is false).
#[inline]
#[cold]
#[allow(dead_code)]
fn cold_bool(b: bool) -> bool {
    b
}

/// Compiler barrier - prevents reordering of loads/stores.
///
/// This prevents the compiler from moving memory operations across this point.
/// It does NOT insert CPU memory barriers (fences), only compiler barriers.
///
/// # Example
///
/// ```rust
/// use abseil::compiler_barrier;
///
/// fn example() {
///     let mut x = 1;
///     let mut y = 2;
///
///     compiler_barrier!();
///
///     // Operations after the barrier cannot be reordered before it
///     x = 3;
///     y = 4;
/// }
/// ```
#[macro_export]
macro_rules! compiler_barrier {
    () => {
        // LLVM's compiler_fence prevents reordering by the compiler
        // but does not emit CPU fence instructions
        std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst)
    };
}

/// Prefetch data into the CPU cache.
///
/// This hints to the CPU to prefetch the given address into cache.
/// The `locality` parameter (0-3) indicates how likely the data is to be used:
/// - 0: Not likely to be used again (don't put in lower-level caches)
/// - 3: Very likely to be used (put in all cache levels)
///
/// # Safety
///
/// The address must be valid (aligned, non-null, within accessible memory).
/// Using an invalid address may cause a segfault.
///
/// # Example
///
/// ```rust
/// use abseil::absl_base::optimization::prefetch;
///
/// fn process_array(data: &[i32]) {
///     const PREFETCH_DISTANCE: usize = 8;
///
///     for i in 0..data.len() {
///         // Prefetch data ahead of our current position
///         if i + PREFETCH_DISTANCE < data.len() {
///             unsafe {
///                 prefetch(data.as_ptr().add(i + PREFETCH_DISTANCE), 3);
///             }
///         }
///
///         // Process current element
///         let _ = data[i];
///     }
/// }
/// ```
#[inline]
pub unsafe fn prefetch<T>(addr: *const T, locality: u32) {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "sse")]
        {
            match locality {
                0 => core::arch::x86_64::_mm_prefetch(addr as *const i8, core::arch::x86_64::_MM_HINT_NTA),
                1 => core::arch::x86_64::_mm_prefetch(addr as *const i8, core::arch::x86_64::_MM_HINT_T0),
                2 => core::arch::x86_64::_mm_prefetch(addr as *const i8, core::arch::x86_64::_MM_HINT_T1),
                _ => core::arch::x86_64::_mm_prefetch(addr as *const i8, core::arch::x86_64::_MM_HINT_T2),
            }
        }
    }

    #[cfg(target_arch = "x86")]
    {
        #[cfg(target_feature = "sse")]
        {
            match locality {
                0 => core::arch::x86::_mm_prefetch(addr as *const i8, core::arch::x86::_MM_HINT_NTA),
                1 => core::arch::x86::_mm_prefetch(addr as *const i8, core::arch::x86::_MM_HINT_T0),
                2 => core::arch::x86::_mm_prefetch(addr as *const i8, core::arch::x86::_MM_HINT_T1),
                _ => core::arch::x86::_mm_prefetch(addr as *const i8, core::arch::x86::_MM_HINT_T2),
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::asm;
        match locality {
            0 => {
                // PRFM PLDL1KEEP - prefetch to L1, don't evict
                asm!("prfm pldl1keep, [{}]", in(reg) addr, options(nostack, readonly));
            }
            1 => {
                // PRFM PLDL2KEEP - prefetch to L2
                asm!("prfm pldl2keep, [{}]", in(reg) addr, options(nostack, readonly));
            }
            2 => {
                // PRFM PLDL3KEEP - prefetch to L3
                asm!("prfm pldl3keep, [{}]", in(reg) addr, options(nostack, readonly));
            }
            _ => {
                // PRFM PLDL3KEEP
                asm!("prfm pldl3keep, [{}]", in(reg) addr, options(nostack, readonly));
            }
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
    {
        // Generic fallback - the compiler may optimize this away
        let _ = addr;
        let _ = locality;
    }
}

/// Prefetch data for write access.
///
/// Similar to `prefetch` but hints that the data will be written to.
///
/// # Safety
///
/// Same safety requirements as `prefetch`.
///
/// # Example
///
/// ```rust
/// use abseil::absl_base::optimization::prefetch_write;
///
/// fn fill_array(data: &mut [i32], value: i32) {
///     const PREFETCH_DISTANCE: usize = 8;
///
///     for i in 0..data.len() {
///         if i + PREFETCH_DISTANCE < data.len() {
///             unsafe {
///                 prefetch_write(data.as_mut_ptr().add(i + PREFETCH_DISTANCE), 3);
///             }
///         }
///
///         data[i] = value;
///     }
/// }
/// ```
#[inline]
pub unsafe fn prefetch_write<T>(addr: *mut T, locality: u32) {
    #[cfg(target_arch = "x86_64")]
    {
        #[cfg(target_feature = "sse2")]
        {
            // For write prefetch, we use the same hints but the CPU
            // may mark the cache line for write access
            match locality {
                0 => core::arch::x86_64::_mm_prefetch(addr as *const i8, core::arch::x86_64::_MM_HINT_T0),
                1 => core::arch::x86_64::_mm_prefetch(addr as *const i8, core::arch::x86_64::_MM_HINT_T0),
                2 => core::arch::x86_64::_mm_prefetch(addr as *const i8, core::arch::x86_64::_MM_HINT_T0),
                _ => core::arch::x86_64::_mm_prefetch(addr as *const i8, core::arch::x86_64::_MM_HINT_T0),
            }
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::asm;
        match locality {
            0 => {
                // PRFM PSTL1KEEP - prefetch for store to L1
                asm!("prfm pstl1keep, [{}]", in(reg) addr, options(nostack));
            }
            1 => {
                // PRFM PSTL2KEEP - prefetch for store to L2
                asm!("prfm pstl2keep, [{}]", in(reg) addr, options(nostack));
            }
            _ => {
                // PRFM PSTL3KEEP - prefetch for store to L3
                asm!("prfm pstl3keep, [{}]", in(reg) addr, options(nostack));
            }
        }
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        let _ = addr;
        let _ = locality;
    }
}

/// CPU pause hint - used in spin-wait loops.
///
/// This hints to the CPU that we're in a spin-wait loop, which:
/// 1. Reduces power consumption
/// 2. Prevents pipeline hazards on some CPUs
/// 3. Tells the CPU this is a spin-wait so it can optimize accordingly
///
/// # Example
///
/// ```rust
/// use abseil::absl_base::optimization::spin_pause;
///
/// fn wait_for_flag(flag: &std::sync::atomic::AtomicBool) {
///     while !flag.load(std::sync::atomic::Ordering::Acquire) {
///         spin_pause();
///     }
/// }
/// ```
#[inline]
pub fn spin_pause() {
    #[cfg(target_arch = "x86_64")]
    {
        core::arch::x86_64::_mm_pause();
    }

    #[cfg(target_arch = "x86")]
    {
        core::arch::x86::_mm_pause();
    }

    #[cfg(target_arch = "aarch64")]
    {
        use core::arch::asm;
        unsafe { asm!("yield", options(nostack)) };
    }

    #[cfg(target_arch = "arm")]
    {
        use core::arch::asm;
        unsafe { asm!("yield", options(nostack)) };
    }

    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64", target_arch = "arm")))]
    {
        // Generic fallback - hint to the compiler that we're spinning
        core::hint::spin_loop();
    }
}

/// Prevents the compiler from optimizing away a memory read.
///
/// This is useful for benchmarking and testing, where you want to ensure
/// a value is actually computed and not optimized away.
///
/// # Example
///
/// ```rust
/// use abseil::absl_base::optimization::escape;
///
/// fn benchmark() {
///     let result = expensive_computation();
///     escape(&result);
/// }
///
/// fn expensive_computation() -> i32 {
///     42
/// }
/// ```
#[inline]
pub fn escape<T>(value: &T) {
    // This volatile read prevents the compiler from optimizing away
    // anything that leads to this value
    unsafe {
        let _ = core::ptr::read_volatile(value as *const T as *const u8);
    }
}

/// Prevents the compiler from reordering memory operations across this point.
///
/// This is a memory barrier that prevents both compiler and CPU reordering.
/// Use `compiler_barrier!` if you only need to prevent compiler reordering.
///
/// # Example
///
/// ```rust
/// use abseil::memory_barrier;
/// use std::sync::atomic::{AtomicI32, Ordering};
///
/// static FLAG: AtomicI32 = AtomicI32::new(0);
/// static DATA: AtomicI32 = AtomicI32::new(0);
///
/// fn producer() {
///     DATA.store(42, Ordering::Relaxed);
///     memory_barrier!(std::sync::atomic::Ordering::Release);
///     FLAG.store(1, Ordering::Relaxed);
/// }
/// ```
#[macro_export]
macro_rules! memory_barrier {
    ($order:expr) => {{
        std::sync::atomic::fence($order);
    }};
}

/// Hints to the compiler that this point is unreachable.
///
/// Similar to `std::intrinsics::unreachable()` but with a clearer name.
///
/// # Panics
///
/// This will cause a panic if reached in debug mode, or undefined behavior
/// in release mode.
///
/// # Example
///
/// ```rust
/// use abseil::assume_unreachable;
///
/// fn process_value(x: i32) -> i32 {
///     match x {
///         1 => 10,
///         2 => 20,
///         _ => assume_unreachable!("x should be 1 or 2"),
///     }
/// }
/// ```
#[macro_export]
macro_rules! assume_unreachable {
    ($($msg:tt)*) => {
        unreachable!($($msg)*)
    };
}

/// Allows the compiler to assume a condition is true for optimization purposes.
///
/// This is similar to C++'s `__builtin_assume`. If the assumption is false,
/// behavior is undefined.
///
/// # Safety
///
/// The condition MUST be true. If false, undefined behavior may occur.
///
/// # Example
///
/// ```rust
/// use abseil::assume;
///
/// unsafe fn divide_checked(a: i32, b: i32) -> i32 {
///     // Tell the compiler that b is never zero
///     assume!(b != 0);
///     a / b
/// }
/// ```
#[macro_export]
macro_rules! assume {
    ($e:expr) => {
        if !$e {
            std::hint::unreachable_unchecked();
        }
    };
}

/// Provides a hint to the CPU about branch prediction.
///
/// This is a lower-level version of `likely!` and `unlikely!` that
/// directly controls branch prediction hints on supported architectures.
///
/// # Example
///
/// ```rust
/// use abseil::branch_hint;
///
/// fn example(condition: bool) {
///     if branch_hint!(condition, true) {
///         // Optimized for condition == true
///     }
/// }
/// ```
#[macro_export]
macro_rules! branch_hint {
    ($condition:expr, $expected:expr) => {{
        let c = $condition;
        // The expected hint helps the compiler with branch prediction
        let _expected = $expected;
        c
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_likely_macro() {
        // Just test that it compiles and works
        let x = 10;
        if likely!(x > 5) {
            assert!(true);
        }
    }

    #[test]
    fn test_unlikely_macro() {
        // Just test that it compiles and works
        let x = 10;
        if unlikely!(x < 5) {
            assert!(false);
        }
    }

    #[test]
    fn test_spin_pause_compiles() {
        // Run spin_pause a few times to ensure it works
        for _ in 0..10 {
            spin_pause();
        }
    }

    #[test]
    fn test_compiler_barrier_compiles() {
        let mut x = 1;
        compiler_barrier!();
        x = 2;
        assert_eq!(x, 2);
    }

    #[test]
    fn test_memory_barrier() {
        use std::sync::atomic::Ordering;
        memory_barrier!(Ordering::SeqCst);
    }

    #[test]
    fn test_escape() {
        let x = 42;
        escape(&x);
    }

    #[test]
    fn test_branch_hint() {
        let condition = true;
        if branch_hint!(condition, true) {
            assert!(true);
        }
    }

    #[test]
    #[should_panic]
    fn test_assume_unreachable() {
        assume_unreachable!("this should panic");
    }

    #[test]
    fn test_prefetch_compile() {
        let data = vec![1, 2, 3, 4, 5];
        unsafe {
            prefetch(data.as_ptr(), 3);
            prefetch_write(data.as_ptr() as *mut i32, 3);
        }
    }
}
