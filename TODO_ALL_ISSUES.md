# Master TODO List - All Issues

## Executive Summary

Comprehensive audit of Bitvue video analyzer codebase completed on 2025-01-30.

### Total Issues Found
- **CRITICAL**: 5 issues (30-40 hours)
- **HIGH**: 13 issues (40-60 hours)
- **MEDIUM**: 20+ issues (60-80 hours)
- **Total**: 38+ issues, 130-180 hours of work

### Issue Breakdown by Type
| Type | Critical | High | Medium | Total |
|------|----------|------|--------|-------|
| Security | 1 | 0 | 4 | 5 |
| Bugs | 3 | 6 | 6 | 15 |
| Performance | 0 | 2 | 5 | 7 |
| Refactoring | 1 | 5 | 5+ | 11+ |

---

## Quick Reference

### CRITICAL Issues (Fix Immediately)
1. [Unsafe mmap TOCTOU](TODO_CRITICAL.md#1-unsafe-mmap-toctou-race-condition) - byte_cache.rs:100-105
2. [LEB128 overflow edge case](TODO_CRITICAL.md#2-bug-integer-overflow-in-leb128-decoding) - leb128.rs:46-51
3. [CDF array validation](TODO_CRITICAL.md#3-bug-missing-cdf-array-validation) - arithmetic.rs:129-149
4. [OBU unchecked slice](TODO_CRITICAL.md#4-bug-unchecked-slice-index-in-obu-parsing) - obu.rs:228-234
5. [Overlay extraction duplication](TODO_CRITICAL.md#5-refactoring-overlay-extraction-code-duplication) - ~1500 LOC

### HIGH Priority Issues
**Performance:**
1. [CodingUnit clone (300 MB/sec)](TODO_HIGH.md#1-codingunit-clone-in-mv-predictor) - coding_unit.rs:377
2. [String allocations in tooltips](TODO_HIGH.md#2-string-allocations-in-tooltips) - parser.rs:280-287

**Bugs:**
3. [Division by zero potential](TODO_HIGH.md#3-division-by-zero-potential-in-block_size_log2) - partition.rs:271-276
4. [Unbounded vector growth](TODO_HIGH.md#4-unbounded-vector-growth-in-frame-header-parsing) - frame_header.rs:104-109
5. [MP4 32-bit overflow](TODO_HIGH.md#5-mp4-integer-overflow-on-32-bit-systems) - mp4.rs:244-246
6. [MKV VINT validation](TODO_HIGH.md#6-mkv-unvalidated-vint-length) - mkv.rs:40-47
7. [TS parser overflow](TODO_HIGH.md#7-ts-parser-integer-overflow-in-offset-calculation) - ts.rs:129-136
8. [Arithmetic decoder underflow](TODO_HIGH.md#8-arithmetic-decoder-potential-underflow-in-refill) - arithmetic.rs:213-244

**Refactoring:**
9. [Inconsistent error types](TODO_HIGH.md#9-inconsistent-error-types) - Multiple files
10. [Frame extraction duplication](TODO_HIGH.md#10-frame-extraction-duplication) - ~400 LOC
11. [Excessive nesting](TODO_HIGH.md#11-excessive-nesting-in-decode_ivf) - decoder.rs:354-391
12. [Complex boolean expressions](TODO_HIGH.md#12-complex-boolean-expressions) - Multiple files
13. [Missing FrameTypeTrait](TODO_HIGH.md#13-missing-frametypetrait-abstraction) - Multiple files

### MEDIUM Priority Issues
**Security:** 4 issues
**Performance:** 5 issues
**Bugs:** 6 issues
**Refactoring:** 5+ issues

See [TODO_MEDIUM.md](TODO_MEDIUM.md) for complete details.

---

## Recommended Fix Order

### Phase 1: CRITICAL Security (3-5 hours)
```bash
# Quick wins that fix security boundaries
1. TODO_CRITICAL.md#2 - LEB128 overflow (1-2h) ⚡
2. TODO_CRITICAL.md#3 - CDF validation (2-3h) ⚡
3. TODO_CRITICAL.md#4 - OBU parsing (2-3h) ⚡
```

### Phase 2: CRITICAL Bugs & Documentation (8-10 hours)
```bash
1. TODO_CRITICAL.md#1 - mmap TOCTOU docs (4-6h)
2. TODO_CRITICAL.md#5 - Overlay extraction (16-24h)  # Can be deferred
```

### Phase 3: HIGH Performance (3-5 hours)
```bash
1. TODO_HIGH.md#1 - CodingUnit clone (1-2h) ⚡
2. TODO_HIGH.md#2 - String allocations (2-3h) ⚡
```

### Phase 4: HIGH Bugs (15-20 hours)
```bash
1. TODO_HIGH.md#3 - Division by zero (1-2h)
2. TODO_HIGH.md#6 - MKV VINT (1-2h)
3. TODO_HIGH.md#7 - TS overflow (1-2h)
4. TODO_HIGH.md#5 - MP4 32-bit (2-3h)
5. TODO_HIGH.md#8 - Arithmetic refill (2-3h)
6. TODO_HIGH.md#4 - Frame header (3-4h)
```

### Phase 5: HIGH Refactoring (20-30 hours)
```bash
1. TODO_HIGH.md#9 - Error types (8-12h)
2. TODO_HIGH.md#10 - Frame builders (6-8h)
3. TODO_HIGH.md#13 - FrameTypeTrait (4-6h)
4. TODO_HIGH.md#11 - Nesting (2-3h)
5. TODO_HIGH.md#12 - Boolean helpers (2-3h)
```

### Phase 6: MEDIUM Issues (60-80 hours)
```bash
# Address as time permits, priority by category
- Security first (4 issues, ~8h)
- Performance second (5 issues, ~15h)
- Bugs third (6 issues, ~18h)
- Refactoring last (5+ issues, ~20h)
```

---

## Quick Wins (Under 3 hours each)

| Issue | Impact | Effort | File |
|-------|--------|--------|------|
| LEB128 overflow check | Security | 1-2h | leb128.rs:46 |
| CodingUnit clone | 300 MB/sec | 1-2h | coding_unit.rs:377 |
| Division by zero | Stability | 1-2h | partition.rs:271 |
| MKV VINT validation | Correctness | 1-2h | mkv.rs:40 |
| TS parser overflow | Safety | 1-2h | ts.rs:129 |
| Cache Arc wrapper | 3 MB/hit | 1-2h | cache.rs:73 |
| Error propagation | Debugging | 1-2h | parser.rs:101 |
| Debug assertions | Release safety | 1-2h | cdf.rs:230 |
| Unnecessary .collect() | Performance | 2-3h | Multiple |
| Complex boolean helpers | Readability | 2-3h | Multiple |

**Total: 10 quick wins, 15-25 hours**

---

## Impact Estimates

### Security Fixes
- **Prevent**: OOM attacks, integer overflow exploits
- **Impact**: Security boundary enforcement
- **Confidence**: High after fixes

### Performance Fixes
- **CodingUnit clone**: Eliminate 300 MB/sec allocations
- **String allocations**: Reduce GC pressure
- **Cache improvements**: 5-10x faster hashing
- **Expected overall improvement**: 2-3x overlay extraction

### Stability Fixes
- **Division by zero**: Prevent panics
- **CDF validation**: Prevent crashes on malformed data
- **Recursion limits**: Prevent stack overflow
- **Expected impact**: Fewer crashes, better error messages

### Maintainability Fixes
- **Code duplication**: Eliminate 2000+ LOC
- **Error types**: Consistent error handling
- **Newtype patterns**: Compile-time validation
- **Expected impact**: Easier to add features, fewer bugs

---

## Testing Strategy

### For Each Fix
- [ ] Unit test demonstrating the issue
- [ ] Implement fix
- [ ] Verify test passes
- [ ] Add regression test
- [ ] Run full test suite
- [ ] Manual testing with edge cases
- [ ] Update documentation if needed

### Fuzzing Targets (Already Exist)
- `fuzz/fuzz_targets/obu_parser.rs`
- `fuzz/fuzz_targets/ivf_parser.rs`
- `fuzz/fuzz_targets/leb128.rs`

### Recommended Additions
- MP4 box parser fuzzer
- Frame header parser fuzzer
- Property-based tests for boundaries

---

## Completion Criteria

### CRITICAL Issues
All CRITICAL issues must be completed before:
- [ ] Any feature work
- [ ] Production deployment
- [ ] Security review sign-off

### HIGH Issues
All HIGH issues should be completed:
- [ ] Before next major release
- [ ] Before adding new codecs
- [ ] Before performance optimizations

### MEDIUM Issues
MEDIUM issues can be addressed incrementally:
- [ ] As time permits between features
- [ ] During dedicated bug fix sprints
- [ ] Based on user feedback priorities

---

## Audit Methodology

This comprehensive audit was conducted using 4 specialized agents:

1. **Security Auditor**: Analyzed code for vulnerabilities, focusing on:
   - Buffer overflows and out-of-bounds access
   - Integer overflow/underflow
   - Resource consumption (DoS)
   - Concurrency issues

2. **Performance Profiler**: Identified bottlenecks in:
   - Hot path allocations
   - Algorithmic complexity
   - Memory usage patterns
   - I/O operations

3. **Code Reviewer**: Found refactoring opportunities:
   - Code duplication
   - Code smells
   - API design issues
   - Type system improvements

4. **Bug Hunter**: Searched for:
   - Logic errors
   - Resource management issues
   - Type mismatches
   - Test gaps

---

## Related Documents

- [SECURITY.md](SECURITY.md) - Security documentation
- [TODO_CRITICAL.md](TODO_CRITICAL.md) - Critical issues detailed
- [TODO_HIGH.md](TODO_HIGH.md) - High priority issues detailed
- [TODO_MEDIUM.md](TODO_MEDIUM.md) - Medium priority issues detailed
- [TODO_PERFORMANCE_REMAINING.md](TODO_PERFORMANCE_REMAINING.md) - Previous performance TODO
- [TODO_REFACTORING.md](TODO_REFACTORING.md) - Previous refactoring TODO

---

## Next Steps

1. **Review and prioritize**: Confirm issue priorities with team
2. **Assign issues**: Distribute work among contributors
3. **Create sprint plan**: Break down into manageable chunks
4. **Set up CI checks**: Prevent regressions
5. **Track progress**: Update TODO files as issues are resolved

---

**Last Updated**: 2025-01-30
**Audit Completed By**: Claude (Security/Performance/Code Review/Bug Agents)
**Version**: 1.0
