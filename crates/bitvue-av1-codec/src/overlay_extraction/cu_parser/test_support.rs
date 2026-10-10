//! Fixture access shared by the `cu_parser` test modules.

pub(super) const AV1_IVF_FIXTURE: &[u8] = include_bytes!("../../../../../test_data/av1_test.ivf");

pub(super) fn find_seq_header_bytes(frames: &[crate::ivf::IvfFrame]) -> Option<Vec<u8>> {
    for frame in frames.iter().take(8) {
        let mut iter = crate::obu::ObuIterator::new(&frame.data);
        while let Some(Ok(found)) = iter.next_obu_with_offset() {
            if found.obu.header.obu_type == crate::obu::ObuType::SequenceHeader {
                return Some(frame.data[found.offset..found.offset + found.consumed].to_vec());
            }
        }
    }
    None
}
