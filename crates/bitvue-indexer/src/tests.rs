use crate::index_stream;
use bitvue_engine::{Command, Core, StreamId};
use std::io::Write;

const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../test_data/av1_test.ivf");

/// Writes the embedded fixture to a real temp file and opens it on the given `Core`/stream --
/// `index_stream` reads through `StreamState.byte_cache`, which is only populated by a real
/// `OpenFile` command (mirrors how every other Core-level test in this workspace sets up state).
fn open_fixture(core: &Core, stream: StreamId) {
    let mut file = tempfile::NamedTempFile::new().expect("create temp file");
    file.write_all(AV1_IVF_FIXTURE)
        .expect("write fixture bytes");
    let events = core.handle_command(Command::OpenFile {
        stream,
        path: file.path().to_path_buf(),
    });
    assert!(
        events
            .iter()
            .any(|e| matches!(e, bitvue_engine::Event::ModelUpdated { .. })),
        "OpenFile should succeed against a real fixture: {events:?}"
    );
    // Keep the temp file alive for the duration of the test by leaking it -- ByteCache holds
    // an mmap/open handle to the path, and NamedTempFile deletes on drop.
    std::mem::forget(file);
}

#[test]
fn index_stream_populates_container_and_units_for_real_av1_ivf() {
    let core = Core::new();
    open_fixture(&core, StreamId::A);

    let events = index_stream(&core, StreamId::A);

    assert!(
        events.iter().any(|e| matches!(
            e,
            bitvue_engine::Event::ModelUpdated {
                kind: bitvue_engine::ModelKind::Container,
                ..
            }
        )),
        "expected a Container ModelUpdated event: {events:?}"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            bitvue_engine::Event::ModelUpdated {
                kind: bitvue_engine::ModelKind::Units,
                ..
            }
        )),
        "expected a Units ModelUpdated event: {events:?}"
    );

    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();

    let container = state
        .container
        .as_ref()
        .expect("container should be populated");
    assert_eq!(container.codec, "av1");
    assert_eq!(container.format, bitvue_engine::ContainerFormat::Ivf);
    assert!(container.width.unwrap_or(0) > 0);
    assert!(container.height.unwrap_or(0) > 0);

    let units = state.units.as_ref().expect("units should be populated");
    assert!(units.unit_count > 0, "expected at least one parsed unit");
    assert_eq!(units.unit_count, units.units.len());
    assert_eq!(units.frame_count, units.unit_count);

    // First frame of a well-formed stream should be a keyframe.
    let first = &units.units[0];
    assert_eq!(first.frame_index, Some(0));
    assert_eq!(first.frame_type.as_deref(), Some("I"));
    assert_eq!(first.offset, 32, "IVF header is 32 bytes");

    // Offsets must be monotonically increasing and non-overlapping (offset_i+1 == offset_i + size_i).
    for pair in units.units.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        assert_eq!(
            b.offset,
            a.offset + a.size as u64,
            "unit {} and {} overlap or have a gap",
            a.frame_index.unwrap_or(usize::MAX),
            b.frame_index.unwrap_or(usize::MAX)
        );
    }
}

#[test]
fn index_stream_reports_a_diagnostic_when_no_file_is_open() {
    let core = Core::new();
    let events = index_stream(&core, StreamId::A);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, bitvue_engine::Event::DiagnosticAdded { .. })),
        "expected a diagnostic when no file is open: {events:?}"
    );
}

#[test]
fn index_stream_reports_a_diagnostic_for_non_ivf_data() {
    let core = Core::new();
    let mut file = tempfile::NamedTempFile::new().expect("create temp file");
    file.write_all(b"not an ivf file, just some bytes")
        .expect("write bytes");
    core.handle_command(Command::OpenFile {
        stream: StreamId::A,
        path: file.path().to_path_buf(),
    });
    std::mem::forget(file);

    let events = index_stream(&core, StreamId::A);
    assert!(
        events
            .iter()
            .any(|e| matches!(e, bitvue_engine::Event::DiagnosticAdded { .. })),
        "expected an honest diagnostic instead of a fabricated result: {events:?}"
    );
}

#[test]
fn get_frame_syntax_returns_a_real_tree_for_the_first_frame() {
    use crate::get_frame_syntax;

    let core = Core::new();
    open_fixture(&core, StreamId::A);
    index_stream(&core, StreamId::A);

    let model = get_frame_syntax(&core, StreamId::A, 0).expect("get_frame_syntax should succeed");

    assert!(!model.nodes.is_empty(), "expected a non-empty syntax tree");
    let root = model
        .nodes
        .get(&model.root_id)
        .expect("root_id should resolve to a real node");
    assert!(
        !root.children.is_empty(),
        "root should have child fields (obu header etc.)"
    );

    // Written into StreamState.syntax too, so SelectBitRange's on-demand nearest-node lookup has
    // something real to search against afterward.
    let stream_state = core.get_stream(StreamId::A);
    let state = stream_state.read();
    assert!(
        state.syntax.is_some(),
        "get_frame_syntax should populate StreamState.syntax"
    );
}

#[test]
fn get_frame_syntax_errors_honestly_when_index_stream_has_not_run() {
    use crate::get_frame_syntax;

    let core = Core::new();
    open_fixture(&core, StreamId::A);
    // Deliberately not calling index_stream first -- state.units is still None.

    let result = get_frame_syntax(&core, StreamId::A, 0);
    assert!(
        result.is_err(),
        "expected an error, not a panic or fabricated tree"
    );
}

#[test]
fn get_frame_syntax_errors_for_an_out_of_range_frame_index() {
    use crate::get_frame_syntax;

    let core = Core::new();
    open_fixture(&core, StreamId::A);
    index_stream(&core, StreamId::A);

    let result = get_frame_syntax(&core, StreamId::A, 999_999);
    assert!(result.is_err());
}

/// Regression test for a real bug: `parse_obu_syntax`'s `global_offset` param is a BIT offset
/// (see `TrackedBitReader::new`'s doc), not a byte offset -- passing the raw byte offset (an
/// earlier version of `get_frame_syntax` did exactly this) silently makes every node's
/// `bit_range` wrong by a factor of 8, without any error or empty-tree symptom to notice it by.
/// Asserts the actual numeric value against the real, known file layout: frame 0's OBU starts at
/// byte 44 (32-byte IVF header + 12-byte chunk header), so its first field must start at bit 352.
#[test]
fn get_frame_syntax_bit_range_is_a_real_bit_offset_not_a_byte_offset() {
    use crate::get_frame_syntax;

    let core = Core::new();
    open_fixture(&core, StreamId::A);
    index_stream(&core, StreamId::A);
    let model = get_frame_syntax(&core, StreamId::A, 0).expect("get_frame_syntax should succeed");

    // The root node ("obu_0") is a synthetic container spanning the whole tree (bit_range
    // 0..total) -- look up a real leaf field instead of taking a blind min across all nodes.
    let forbidden_bit = model
        .nodes
        .values()
        .find(|n| n.field_name == "obu_forbidden_bit")
        .unwrap_or_else(|| {
            panic!(
                "expected an obu_forbidden_bit node, got: {:?}",
                model.nodes.keys().collect::<Vec<_>>()
            )
        });
    assert_eq!(
        forbidden_bit.bit_range.start_bit, 352,
        "expected obu_forbidden_bit to start at bit 352 (byte 44 * 8) -- got {}, which looks \
         like a byte offset (44) leaking through unmultiplied",
        forbidden_bit.bit_range.start_bit
    );
}
