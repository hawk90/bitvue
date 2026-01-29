# Security Issues - Remaining TODO

## Summary
- **Status**: 7 CRITICAL/HIGH issues fixed ✅
- **Remaining**: 8 issues (6 MEDIUM, 2 LOW)
- **Priority**: Address MEDIUM issues before release

---

## MEDIUM Priority (8 issues)

### 1. Unchecked Slice Indexing
**Risk**: Potential panic on malformed input
**Locations**:
- `crates/bitvue-decode/src/decoder.rs:364-381` - IVF timestamp extraction
- `crates/bitvue-av1/src/parser.rs` - OBU header parsing
- `crates/bitvue-decode/src/vvdec.rs` - NAL unit parsing

**Fix**:
```rust
// Replace:
let value = data[offset];

// With:
let value = data.get(offset).ok_or(DecodeError::...)?;
```

**Estimated effort**: 2-3 hours
**Impact**: Prevents panics on malformed files

---

### 2. Potential Panic in Error Paths
**Risk**: Error handling that might panic instead of returning Result
**Locations**:
- `crates/bitvue-decode/src/vvdec.rs:778` - Drop implementation error handling
- `crates/bitvue-av1/src/parser.rs` - Parse error recovery

**Fix**:
- Review all `.unwrap()`, `.expect()`, `.panic!()` calls
- Replace with proper error propagation
- Add `#![forbid(unwrap_used)]` lint

**Estimated effort**: 3-4 hours
**Impact**: More robust error handling

---

### 3. Resource Limits Validation
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

### 4. Error Message Information Leakage
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

### 5. Input Validation Edge Cases
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

### 6. Dependency Vulnerabilities
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

### 8. Documentation of Security Assumptions
**Risk**: Security properties not documented
**Action Required**:
- Document threat model
- Document security assumptions
- Document safe usage patterns
- Add SECURITY.md with vulnerability reporting

**Estimated effort**: 3-4 hours
**Impact**: Better security communication

---

### 9. Fuzzing Test Coverage
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

1. **#1 - Unchecked indexing** (high impact, common vulnerability)
2. **#3 - Resource limits** (easy fixes, good protection)
3. **#7 - Debug assertions** (simple audit, good safety)
4. **#4 - Error message leakage** (review and sanitize)
5. **#2 - Panic in error paths** (improve robustness)
6. **#5 - Input validation** (comprehensive testing)
7. **#8 - Documentation** (ongoing)
8. **#9 - Fuzzing** (long-term investment)

---

## Completed (7 issues) ✅

1. ✅ Mutex poisoning panics (vvdec.rs)
2. ✅ Unbounded timeout loops (vvdec.rs)
3. ✅ Integer overflow in YUV calculations (decoder.rs)
4. ✅ Unbounded frame iteration (decoder.rs)
5. ✅ Type cast overflow (vvdec.rs)
6. ✅ Thread safety TOCTOU race (cache.rs) - Single lock acquisition pattern
7. ✅ Dependency vulnerabilities - Audit complete, no CVEs found
