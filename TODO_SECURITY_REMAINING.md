# Security Issues - Remaining TODO

## Summary
- **Status**: 10 CRITICAL/HIGH issues fixed ✅
- **Remaining**: 6 issues (4 MEDIUM, 2 LOW)
- **Priority**: Address MEDIUM issues before release

---

## MEDIUM Priority (8 issues)

### 1. Unchecked Slice Indexing ✅
**Status**: Already addressed - all slice access is bounds-checked

**Audit results**:
- decoder.rs: Uses safe `.get()` patterns and validated offsets
- vvdec.rs: No direct slice indexing (FFI bindings only)
- ivf.rs: All direct indexing has bounds checks

---

### 2. Potential Panic in Error Paths ✅
**Status**: Mostly addressed - reviewed all unwrap/expect calls

**Audit results**:
- All unwrap/expect in production code are either:
  - In test code (acceptable)
  - On LazyLock mutexes (panic on poison is correct behavior)
  - Fixed: rgb_to_image now returns Result instead of expect()

**Remaining**: None - all panic-prone code is in appropriate contexts

---

### 3. Resource Limits Validation ✅
**Status**: Addressed - added cache size limits

**Implemented**:
- Added MAX_CACHE_ENTRIES constant (64) to coding unit cache
- Eviction logic removes 25% of entries when limit reached
- Existing dimension limits (8K max) validated in PlaneConfig
- No thread pool or other configurable limits found

---

### 4. Error Message Information Leakage
**Risk**: Missing validation for user-configurable limits
**Locations**:
- Thread pool sizes
- Cache sizes
- Buffer sizes

**Fix**:
```rust
pub fn set_thread_count(count: usize) -> Result<()> {
    const MAX_THREADS: usize = 64;
    if count == 0 || count > MAX_THREADS {
        return Err(Error::InvalidConfig);
    }
    // ...
}
```

**Estimated effort**: 2 hours
**Impact**: Prevents resource exhaustion

---

### 3. Error Message Information Leakage
**Risk**: Detailed errors might expose internal state
**Locations**:
- All `DecodeError::Decode(format!(...))` messages
- Debug output in error messages

**Fix**:
- Review all error messages for sensitive info
- Add separate debug vs user-facing messages
- Sanitize file paths in errors
- Remove internal offsets/pointers from public errors

**Estimated effort**: 2-3 hours
**Impact**: Better security posture

---

### 4. Input Validation Edge Cases
**Risk**: Corner cases in validation logic
**Issues**:
- Zero-size dimensions after arithmetic
- Overflow in dimension calculations
- Negative values cast to unsigned

**Fix**:
- Add property-based tests
- Test boundary values (0, 1, MAX)
- Test overflow scenarios
- Add fuzzing targets

**Estimated effort**: 4-5 hours
**Impact**: More robust validation

---

### 5. Debug Assertions in Release
**Risk**: External crates might have known CVEs
**Action Required**:
```bash
cargo audit
cargo outdated
cargo update
```

**Fix**:
- Run `cargo audit` and fix findings
- Update dependencies to latest secure versions
- Add CI job for automated audit
- Review security advisories for dav1d, vvdec

**Estimated effort**: 2-3 hours
**Impact**: Address known vulnerabilities

---

### 7. Debug Assertions in Release
**Risk**: Important checks only run in debug builds
**Locations**:
- `debug_assert!()` calls that should be `assert!()`
- Validation skipped in release mode

**Fix**:
```rust
// Change critical debug_assert! to assert!
debug_assert!(offset < data.len()); // ❌
assert!(offset < data.len());       // ✅
```

**Estimated effort**: 1-2 hours
**Impact**: Better release build safety

---

## LOW Priority (2 issues)

### 6. Documentation of Security Assumptions
**Risk**: Security properties not documented
**Action Required**:
- Document threat model
- Document security assumptions
- Document safe usage patterns
- Add SECURITY.md with vulnerability reporting

**Estimated effort**: 3-4 hours
**Impact**: Better security communication

---

### 7. Fuzzing Test Coverage
**Risk**: Parser bugs not caught by unit tests
**Action Required**:
- Add cargo-fuzz targets for:
  - AV1 OBU parser
  - IVF parser
  - VVC NAL parser
  - Frame header parser
- Run fuzzing in CI

**Estimated effort**: 6-8 hours
**Impact**: Find hidden bugs

---

## Testing Checklist

For each fix:
- [ ] Add unit test demonstrating the issue
- [ ] Implement fix
- [ ] Verify test passes
- [ ] Add regression test
- [ ] Update documentation
- [ ] Run full test suite
- [ ] Manual testing with edge cases

---

## Priority Order

1. **#5 - Debug assertions** (simple audit, good safety)
2. **#4 - Error message leakage** (review and sanitize)
3. **#6 - Input validation** (comprehensive testing)
4. **#7 - Documentation** (ongoing)
5. **#8 - Fuzzing** (long-term investment)

---

## Completed (10 issues) ✅

1. ✅ Mutex poisoning panics (vvdec.rs)
2. ✅ Unbounded timeout loops (vvdec.rs)
3. ✅ Integer overflow in YUV calculations (decoder.rs)
4. ✅ Unbounded frame iteration (decoder.rs)
5. ✅ Type cast overflow (vvdec.rs)
6. ✅ Thread safety TOCTOU race (cache.rs) - Single lock acquisition pattern
7. ✅ Dependency vulnerabilities - Audit complete, no CVEs found
8. ✅ Unchecked slice indexing - Audit complete, all access bounds-checked
9. ✅ Panic in error paths - Fixed rgb_to_image, reviewed all unwrap/expect
10. ✅ Resource limits validation - Added cache size limit, reviewed all limits
