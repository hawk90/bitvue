//! The cap must abort the process (not return null, not hang) and say why.

use bitvue_test_alloc::CappedAlloc;
use std::process::Command;

#[global_allocator]
static ALLOC: CappedAlloc = CappedAlloc::new(64 * 1024 * 1024);

const CHILD_ENV: &str = "BITVUE_TEST_ALLOC_CHILD";

#[test]
fn child_allocates_past_the_cap() {
    if std::env::var_os(CHILD_ENV).is_none() {
        return; // only meaningful when launched by the parent test below
    }
    let mut v: Vec<Vec<u8>> = Vec::new();
    loop {
        v.push(vec![1u8; 8 * 1024 * 1024]); // endless growth, like a runaway mutant
    }
}

#[test]
fn a_runaway_allocation_aborts_with_a_test_alloc_message() {
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "child_allocates_past_the_cap", "--nocapture"])
        .env(CHILD_ENV, "1")
        .output()
        .unwrap();
    assert!(!out.status.success(), "child should have been aborted");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("TEST_ALLOC: aborting"), "stderr: {stderr}");
}

#[test]
fn allocation_under_the_cap_is_untouched() {
    let v = vec![7u8; 16 * 1024 * 1024];
    assert_eq!(
        v.iter().map(|&b| u64::from(b)).sum::<u64>(),
        7 * 16 * 1024 * 1024
    );
}
