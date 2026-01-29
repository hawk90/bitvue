# Bitvue Master TODO List

## Overview

This document provides a comprehensive overview of all remaining tasks for the Bitvue video analyzer project.

**Last Updated**: 2026-01-30
**Status**: 10 issues fixed (5 security, 5 performance), 24 remaining

---

## Completed Work ✅

### Security Fixes (5/15)
1. ✅ **CRITICAL**: Mutex poisoning panics in vvdec.rs
2. ✅ **CRITICAL**: Unbounded timeout loops with thread abandonment
3. ✅ **HIGH**: Integer overflow in YUV size calculations
4. ✅ **HIGH**: Unbounded frame iteration DoS attack vector
5. ✅ **HIGH**: Type cast overflow in VVC decoder

**Commits**:
- Security fixes: Multiple commits in bitvue-decode/src/vvdec.rs and decoder.rs
- All 148 decoder tests passing

### Performance Optimizations (5/9)
1. ✅ **CRITICAL**: Eliminated tile_data clones (1-10 MB) with Arc (commit 18df360)
2. ✅ **CRITICAL**: Eliminated O(n²) CU lookups with spatial index (commit d84d7ae)
3. ✅ **CRITICAL**: Optimized frame data transfer with send_data_owned (commit 98c9dfc)
4. ✅ **HIGH**: Eliminated 1000+ plane extraction reallocations (commit 387dbe5)
5. ✅ **HIGH**: IVF frame cloning (determined unavoidable due to dav1d API)

**Impact**:
- ~3x faster multi-overlay extraction
- ~250x faster QP/MV extraction for 1080p
- Eliminated 1080 reallocations per frame
- All 17 overlay extraction tests passing

---

## Remaining Work

### 1. Security Issues (10 remaining)
**File**: `TODO_SECURITY_REMAINING.md`

**MEDIUM Priority** (8 issues):
1. Unchecked slice indexing → 2-3 hours
2. Potential panic in error paths → 3-4 hours
3. Thread safety concerns → 4-5 hours
4. Resource limits validation → 2 hours
5. Error message information leakage → 2-3 hours
6. Input validation edge cases → 4-5 hours
7. Dependency vulnerabilities → 2-3 hours ⭐ **Quick Win**
8. Debug assertions in release → 1-2 hours ⭐ **Quick Win**

**LOW Priority** (2 issues):
9. Documentation of security assumptions → 3-4 hours
10. Fuzzing test coverage → 6-8 hours

**Total Estimated Effort**: 30-37 hours

**Priority Order**:
1. #7 - Dependency audit (automated, quick)
2. #1 - Unchecked indexing (common vulnerability)
3. #4 - Resource limits (easy protection)
4. #8 - Debug assertions (simple audit)

---

### 2. Performance Issues (4 remaining)
**File**: `TODO_PERFORMANCE_REMAINING.md`

**MEDIUM Priority** (4 issues):
1. String allocations in hot paths → 3-4 hours
2. Vec resizing in loops → 2-3 hours ⭐ **Quick Win**
3. Redundant clones in non-critical paths → 4-5 hours
4. Debug logging overhead → 2-3 hours ⭐ **Quick Win**

**Optional Optimizations**:
5. SIMD optimizations → 8-10 hours (20-40% speedup)
6. Parallel frame decoding → 10-15 hours (near-linear speedup)
7. Memory pool for frames → 6-8 hours
8. Zero-copy frame export → 10-12 hours (30-50% faster)

**Total Estimated Effort**: 11-15 hours (MEDIUM only)

**Priority Order**:
1. #4 - Debug logging (easy fix)
2. #2 - Vec resizing (straightforward)
3. #1 - String allocations (good impact)

---

### 3. Refactoring Opportunities (15 issues)
**File**: `TODO_REFACTORING.md`

**High Priority** (improve quality):
1. Duplicate parse_all_coding_units → 2 hours ⭐
2. Duplicate spatial index → 1 hour ⭐
3. Magic numbers to constants → 2-3 hours ⭐
4. Consistent error handling → 4-5 hours

**Medium Priority** (improve maintainability):
5. Split decode_ivf function → 3-4 hours
6. Use match for frame types → 2-3 hours
7. Module organization → 3-4 hours
8. API documentation → 10-15 hours

**Low Priority** (nice to have):
9. OBU iterator → 3-4 hours
10. Pixel format enum → 4-5 hours
11. Newtype patterns → 6-8 hours
12. Test organization → 2-3 hours
13. Property-based testing → 8-10 hours

**Total Estimated Effort**: 50-68 hours

**Priority Order**:
1. #1, #2 - Eliminate duplication (prevent bugs)
2. #3 - Named constants (self-documenting)
3. #4 - Consistent errors (better API)

---

## Quick Wins ⭐

Tasks that provide good value for minimal effort:

### Security Quick Wins (4-6 hours total)
- [ ] #7 - Run `cargo audit` and fix findings
- [ ] #8 - Audit debug_assert! usage
- [ ] #4 - Add validation for thread count, cache size, etc.

### Performance Quick Wins (4-6 hours total)
- [ ] #4 - Fix debug logging evaluation
- [ ] #2 - Pre-allocate vectors in loops

### Refactoring Quick Wins (3-4 hours total)
- [ ] #1 - Extract parse_all_coding_units to shared module
- [ ] #2 - Extract spatial index to shared module
- [ ] #3 - Named constants for magic numbers

**Total Quick Wins**: 11-16 hours for significant improvements

---

## Recommended Roadmap

### Phase 1: Security Hardening (1 week)
**Goal**: Production-ready security

1. **Day 1-2**: Quick wins
   - Run cargo audit
   - Fix debug assertions
   - Add resource limit validation

2. **Day 3-4**: Critical security
   - Fix unchecked indexing
   - Sanitize error messages
   - Validate edge cases

3. **Day 5**: Testing
   - Add security tests
   - Manual fuzzing
   - Code review

**Deliverable**: All MEDIUM security issues fixed

---

### Phase 2: Code Quality (1 week)
**Goal**: Maintainable, clean codebase

1. **Day 1**: Eliminate duplication
   - Extract parse_all_coding_units
   - Extract spatial index
   - Named constants

2. **Day 2-3**: Improve structure
   - Split large functions
   - Use match expressions
   - Consistent errors

3. **Day 4-5**: Documentation
   - API documentation
   - Usage examples
   - Architecture docs

**Deliverable**: High-priority refactoring complete

---

### Phase 3: Performance Polish (3-5 days)
**Goal**: Optimized hot paths

1. **Day 1**: Quick wins
   - Fix logging overhead
   - Pre-allocate vectors
   - Profile for low-hanging fruit

2. **Day 2-3**: String optimizations
   - Reduce allocations in hot paths
   - Audit clone usage
   - Optimize error formatting

3. **Day 4-5**: Benchmarking
   - Create benchmark suite
   - Continuous performance monitoring
   - Document performance targets

**Deliverable**: All MEDIUM performance issues fixed

---

### Phase 4: Advanced Features (Optional)
**Goal**: Next-level performance

1. SIMD optimizations (1-2 weeks)
2. Parallel decoding (2-3 weeks)
3. Zero-copy export (2 weeks)
4. Comprehensive fuzzing (ongoing)

**Deliverable**: Industry-leading performance

---

## Testing Strategy

### Security Testing
```bash
# Dependency audit
cargo audit

# Static analysis
cargo clippy -- -D warnings

# Security lints
cargo clippy -- -D clippy::unwrap_used -D clippy::expect_used

# Fuzzing
cargo fuzz run obu_parser
cargo fuzz run ivf_parser
cargo fuzz run frame_header_parser
```

### Performance Testing
```bash
# Benchmarks
cargo bench

# Profiling
cargo flamegraph --bin bitvue-cli -- decode large.ivf

# Memory profiling
cargo instruments --template Allocations

# Continuous monitoring
cargo bench --save-baseline before
# ... make changes ...
cargo bench --baseline before
```

### Code Quality
```bash
# Coverage
cargo tarpaulin --out Html

# Documentation
cargo doc --no-deps --open

# Code metrics
tokei crates/
cargo bloat --release
```

---

## Success Metrics

### Security
- [ ] Zero known vulnerabilities
- [ ] All inputs validated
- [ ] No panics on malformed input
- [ ] Fuzzing runs 24+ hours without crashes

### Performance
- [ ] 1080p decode ≥ 60 fps
- [ ] 4K decode ≥ 15 fps
- [ ] Overlay extraction ≤ 5ms per frame
- [ ] Memory usage ≤ 100MB for 1080p

### Code Quality
- [ ] Test coverage ≥ 80%
- [ ] All public APIs documented
- [ ] No clippy warnings
- [ ] No code duplication

---

## Team Allocation

### Full-Time (1 developer)
- **Week 1**: Security hardening
- **Week 2**: Code quality
- **Week 3**: Performance polish
- **Week 4+**: Advanced features

### Part-Time (multiple contributors)
- **Security lead**: Phase 1
- **Performance engineer**: Phase 3
- **Code quality**: Phase 2 (ongoing)
- **Documentation**: Parallel to all phases

---

## Release Checklist

Before releasing v1.0:

### Security
- [ ] All MEDIUM security issues fixed
- [ ] Dependency audit clean
- [ ] Security documentation complete
- [ ] No unsafe code without justification

### Performance
- [ ] Benchmarks meet targets
- [ ] No performance regressions vs v0.10
- [ ] Profiling shows no obvious hotspots
- [ ] Memory leaks checked

### Quality
- [ ] All tests passing
- [ ] Code coverage ≥ 80%
- [ ] No clippy warnings
- [ ] All public APIs documented

### Process
- [ ] CHANGELOG updated
- [ ] Migration guide (if breaking changes)
- [ ] Release notes written
- [ ] Tagged in git

---

## Contact & Support

- **Primary Maintainer**: [Your Name]
- **Security Issues**: security@bitvue.com (private)
- **Bug Reports**: https://github.com/yourorg/bitvue/issues
- **Documentation**: https://docs.bitvue.com

---

## Files in This TODO System

1. **TODO_MASTER.md** (this file) - Overview and roadmap
2. **TODO_SECURITY_REMAINING.md** - 10 security issues with details
3. **TODO_PERFORMANCE_REMAINING.md** - 4 performance issues with benchmarks
4. **TODO_REFACTORING.md** - 15 code quality improvements

**Usage**:
- Review TODO_MASTER.md for high-level planning
- Dive into specific files for implementation details
- Update completion status as you work
- Commit TODO updates with related code changes
