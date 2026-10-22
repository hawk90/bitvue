//! Test-only global allocator with a hard cap on live heap bytes.
//!
//! Why: cargo-mutants rewrites operators (`offset += n` -> `offset *= n`), which can turn a loop
//! into an endless one that allocates on every pass. On 2026-10-22 such a mutant ate all 32 GB
//! within seconds and kernel-panicked the machine twice. macOS has no cgroups and ignores
//! `ulimit -v`, so the cap is enforced in-process: the allocation that would cross the cap prints
//! a `TEST_ALLOC:` line and aborts. No polling, so no race and no overshoot.
//!
//! Install it in every test binary (lib unit tests and each `tests/*.rs`) with
//! `bitvue_test_alloc::install!();` — `scripts/check_test_alloc.sh` verifies none is missing.
//! `scripts/mem_guard_runner.sh` is the second layer for what this cannot see (mmap, FFI
//! allocations, stack).

use std::alloc::{GlobalAlloc, Layout, System};
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Live-bytes cap. Normal bitvue test binaries peak at tens of MB; 1 GiB leaves ample headroom
/// while stopping a runaway well before it hurts the machine. Deliberately BELOW the 2 GB RSS cap
/// in `scripts/mem_guard_runner.sh`, so this deterministic check always trips first and the
/// polling guard is only the backstop for what the allocator cannot see (mmap, FFI, stack).
pub const DEFAULT_CAP_BYTES: usize = 1024 * 1024 * 1024;

/// `System` allocator that aborts the process when live heap bytes would exceed `cap`.
pub struct CappedAlloc {
    live: AtomicUsize,
    cap: usize,
}

impl CappedAlloc {
    pub const fn new(cap: usize) -> Self {
        Self {
            live: AtomicUsize::new(0),
            cap,
        }
    }

    /// Live heap bytes right now.
    pub fn live_bytes(&self) -> usize {
        self.live.load(Ordering::Relaxed)
    }

    /// Reserve `grow` more bytes, or abort if that crosses the cap.
    fn reserve(&self, grow: usize) {
        let before = self.live.fetch_add(grow, Ordering::Relaxed);
        if before.saturating_add(grow) > self.cap {
            self.live.fetch_sub(grow, Ordering::Relaxed);
            self.abort_over_cap(grow);
        }
    }

    #[cold]
    #[inline(never)]
    fn abort_over_cap(&self, requested: usize) -> ! {
        // Unbuffered stderr: writing does not allocate.
        let _ = writeln!(
            std::io::stderr(),
            "TEST_ALLOC: aborting, a {requested}-byte allocation would pass the {} MB live-heap cap",
            self.cap / (1024 * 1024)
        );
        std::process::abort()
    }
}

unsafe impl GlobalAlloc for CappedAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.reserve(layout.size());
        let p = System.alloc(layout);
        if p.is_null() {
            self.live.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        p
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        self.reserve(layout.size());
        let p = System.alloc_zeroed(layout);
        if p.is_null() {
            self.live.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        p
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        self.live.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old = layout.size();
        if new_size > old {
            self.reserve(new_size - old);
        }
        let p = System.realloc(ptr, layout, new_size);
        if p.is_null() {
            if new_size > old {
                self.live.fetch_sub(new_size - old, Ordering::Relaxed);
            }
        } else if new_size < old {
            self.live.fetch_sub(old - new_size, Ordering::Relaxed);
        }
        p
    }
}

/// Make this test binary use [`CappedAlloc`] with [`DEFAULT_CAP_BYTES`].
#[macro_export]
macro_rules! install {
    () => {
        #[global_allocator]
        static TEST_ALLOC: $crate::CappedAlloc =
            $crate::CappedAlloc::new($crate::DEFAULT_CAP_BYTES);
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_releases_live_bytes() {
        let a = CappedAlloc::new(1 << 20);
        let layout = Layout::from_size_align(4096, 8).unwrap();
        unsafe {
            let p = a.alloc(layout);
            assert!(!p.is_null());
            assert_eq!(a.live_bytes(), 4096);
            let p = a.realloc(p, layout, 8192);
            assert_eq!(a.live_bytes(), 8192);
            let p = a.realloc(p, Layout::from_size_align(8192, 8).unwrap(), 1024);
            assert_eq!(a.live_bytes(), 1024);
            a.dealloc(p, Layout::from_size_align(1024, 8).unwrap());
        }
        assert_eq!(a.live_bytes(), 0);
    }
}
