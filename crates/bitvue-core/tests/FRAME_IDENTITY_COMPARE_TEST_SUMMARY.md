# Frame Identity Compare Test Summary

## Test File Created
**Path**: `/Users/hawk/Workspaces/bitvue/crates/bitvue-core/tests/frame_identity_compare_av1_viz_core_009.rs`

**Deliverable**: cursor:FrameIdentity:Compare:AV1:viz_core  
**Subtask**: S.T0-1.AV1.FrameIdentity.Compare.impl.viz_core.009

## Frame Identity Contract Validation

This test suite validates the FRAME_IDENTITY_CONTRACT in Compare mode:

### PRIMARY INVARIANT
- `display_idx` is the ONLY public frame identifier (PTS-sorted display order)
- `decode_idx` is INTERNAL ONLY and never exposed to UI or cursors

### Compare Mode Contract
- Both streams (A and B) navigate using `display_idx`
- Cursor synchronization MUST use `display_idx` for alignment
- Hover state propagates between streams via `display_idx`
- Selection state synchronizes using `display_idx`
- Frame pair mapping operates on `display_idx` exclusively

## Test Coverage (20 tests)

### 1. Cursor Navigation Synchronization
- ✅ `test_cursor_synchronized_navigation_same_length` - Both streams navigate to same display_idx
- ✅ `test_cursor_synchronized_navigation_with_reordering` - Display_idx navigation with decode/display reordering
- ✅ `test_cursor_navigation_different_stream_lengths` - Cursor behavior when streams have different lengths
- ✅ `test_cursor_navigation_out_of_bounds` - Bounds checking for display_idx navigation
- ✅ `test_cursor_sequential_navigation_display_idx` - Sequential forward navigation
- ✅ `test_cursor_reverse_navigation_display_idx` - Sequential reverse navigation
- ✅ `test_cursor_random_navigation_display_idx` - Random access navigation
- ✅ `test_cursor_independent_stream_navigation` - Independent cursor positions per stream

### 2. Hover State Synchronization
- ✅ `test_hover_stream_a_propagates_to_stream_b` - Hover in stream A syncs to stream B
- ✅ `test_hover_stream_b_propagates_to_stream_a` - Hover in stream B syncs to stream A
- ✅ `test_hover_clear_synchronizes` - Hover clear propagates between streams
- ✅ `test_hover_with_different_stream_lengths` - Hover with mismatched stream lengths
- ✅ `test_hover_and_selection_interaction` - Hover and selection state independence

### 3. Selection Propagation
- ✅ `test_selection_propagation_same_display_idx` - Select synchronized frame pair
- ✅ `test_selection_propagation_different_display_idx` - Select different display_idx per stream
- ✅ `test_selection_clear` - Clear selection maintains cursor positions

### 4. Frame Gaps and Edge Cases
- ✅ `test_cursor_frame_gaps_stream_a` - Navigation with gaps in stream A display_idx
- ✅ `test_cursor_frame_gaps_stream_b` - Navigation with gaps in stream B display_idx

### 5. Display Index Consistency
- ✅ `test_cursor_display_idx_consistency_with_reordering` - Display_idx consistency across decode reordering
- ✅ `test_frame_pair_mapping_validates_display_idx` - Frame pair lookup by display_idx

## Mock Implementation Components

### CompareCursor
Tracks cursor state for both streams:
- `stream_a_display_idx: Option<usize>` - Current position in stream A
- `stream_b_display_idx: Option<usize>` - Current position in stream B
- `stream_a_hover: Option<usize>` - Hover state for stream A
- `stream_b_hover: Option<usize>` - Hover state for stream B
- `selection: Option<(usize, usize)>` - Selected frame pair

### FramePairMapper
Maps frames between streams using display_idx:
- `has_stream_a_frame(display_idx)` - Check frame existence in stream A
- `has_stream_b_frame(display_idx)` - Check frame existence in stream B
- `find_pair(display_idx)` - Find matching frame pair
- `max_display_idx()` - Get maximum display_idx across both streams

### CompareEvidenceManager
Manages compare mode state and cursor operations:
- `navigate_synchronized(display_idx)` - Navigate both streams together
- `navigate_with_bounds(display_idx)` - Navigate with per-stream bounds checking

### FrameMetadata
Test frame data structure:
- `display_idx: usize` - Display order (PUBLIC)
- `decode_idx: usize` - Decode order (INTERNAL ONLY)
- `pts: i64` - Presentation timestamp
- `frame_type: FrameType` - Key or Inter frame

## Test Scenarios

### Decode/Display Reordering
Tests simulate realistic B-frame reordering scenarios where decode order differs from display order:
- Decode: I P B B P
- Display: I B B P P

All cursor operations use `display_idx` regardless of reordering.

### Stream Length Mismatches
- Stream A: 10 frames
- Stream B: 6 frames

Cursor navigation handles gracefully when display_idx exists in one stream but not the other.

### Frame Gaps
Tests handle sparse display_idx sequences:
- Stream A: [0, 1, 2, 5, 6] (missing 3, 4)
- Stream B: [0, 1, 5, 6] (missing 2, 3, 4)

## Contract Validation Points

Each test validates one or more of these contract points:

1. **Display Index Primacy**: Cursor operations ONLY use display_idx
2. **Decode Index Internality**: decode_idx is never exposed to cursor state
3. **Synchronization**: Hover and navigation sync between streams via display_idx
4. **Independence**: Each stream can have independent display_idx cursor position
5. **Bounds Safety**: Navigation respects frame existence per stream
6. **Gap Handling**: Cursor handles sparse display_idx sequences correctly

## Test Results
```
running 20 tests
test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

All tests pass, validating the Frame Identity Contract for Compare mode cursor operations.
