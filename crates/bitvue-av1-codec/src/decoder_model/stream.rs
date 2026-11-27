//! The frames of a stream in decoding order, each with the size of its decodable frame group
//! (spec E.2 "Decodable Frame Group"), the input of the decoder model.

use super::FrameRecord;
use crate::frame_header_full::{parse_frame_header_full_for_layer, RefFrameState};
use crate::{parse_all_obus, parse_sequence_header, ObuType, SequenceHeader};

/// A frame that has started but whose last OBU has not been seen.
struct OpenFrame {
    record: FrameRecord,
    /// Bits since the previous frame ended, up to and including the last OBU of this frame seen so
    /// far (its header OBU, then each tile group OBU).
    bits_through_last_obu: u64,
}

/// Whether `obu` belongs to the operating point `idc` (spec 7.1 `drop_obu`: an OBU with an
/// extension is dropped when the point does not contain its layers; sequence headers and temporal
/// delimiters are always kept).
fn in_operating_point(idc: u16, obu: &crate::Obu) -> bool {
    let header = &obu.header;
    if idc == 0
        || !header.has_extension
        || matches!(
            header.obu_type,
            ObuType::SequenceHeader | ObuType::TemporalDelimiter
        )
    {
        return true;
    }
    (idc >> header.temporal_id) & 1 == 1 && (idc >> (header.spatial_id + 8)) & 1 == 1
}

/// Ends the open frame: its decodable frame group is every OBU up to its last one, and what was
/// read after that belongs to the next frame.
fn close(open: &mut Option<OpenFrame>, pending_bits: &mut u64, records: &mut Vec<FrameRecord>) {
    if let Some(mut frame) = open.take() {
        frame.record.coded_bits = frame.bits_through_last_obu;
        *pending_bits -= frame.bits_through_last_obu;
        records.push(frame.record);
    }
}

/// Reads every packet in order. `Err` carries the reason the stream cannot be read as a model
/// input (a packet that does not parse, a frame before any sequence header).
///
/// The decodable frame group of a frame is every OBU after the last OBU of the previous decoded
/// frame (`show_existing_frame` = 0) through the last tile group of this one: the temporal
/// delimiter, sequence header, metadata and the headers that repeat an earlier frame in between
/// all count towards the frame that follows them. OBUs after the stream's last frame count for no
/// frame.
pub(super) fn read_frames<'a>(
    packets: impl IntoIterator<Item = &'a [u8]>,
) -> Result<(SequenceHeader, Vec<FrameRecord>), String> {
    let mut seq: Option<SequenceHeader> = None;
    let mut ref_state = RefFrameState::new();
    let mut records: Vec<FrameRecord> = Vec::new();
    let mut open: Option<OpenFrame> = None;
    // bits of OBUs read since the last decoded frame ended
    let mut pending_bits: u64 = 0;

    for (packet_index, packet) in packets.into_iter().enumerate() {
        let obus = parse_all_obus(packet).map_err(|e| format!("packet {packet_index}: {e}"))?;
        for obu in &obus {
            let op_idc = seq
                .as_ref()
                .and_then(|s| s.operating_points.first())
                .map_or(0, |op| op.idc);
            if !in_operating_point(op_idc, obu) {
                continue;
            }
            let bits = obu.total_size * 8;
            match obu.header.obu_type {
                ObuType::SequenceHeader => {
                    if seq.is_none() {
                        seq = Some(
                            parse_sequence_header(&obu.payload)
                                .map_err(|e| format!("packet {packet_index}: {e}"))?,
                        );
                    }
                    pending_bits += bits;
                }
                ObuType::Frame | ObuType::FrameHeader => {
                    close(&mut open, &mut pending_bits, &mut records);
                    let seq = seq.as_ref().ok_or_else(|| {
                        format!("packet {packet_index}: a frame before the sequence header")
                    })?;
                    let header = parse_frame_header_full_for_layer(
                        &obu.payload,
                        seq,
                        &mut ref_state,
                        obu.header.temporal_id,
                        obu.header.spatial_id,
                    )
                    .map_err(|e| format!("packet {packet_index}: {e}"))?;
                    pending_bits += bits;
                    let luma_samples = if header.show_existing_frame {
                        0
                    } else if header.frame_type.is_intra() {
                        u64::from(header.upscaled_width) * u64::from(header.height)
                    } else {
                        u64::from(seq.max_frame_width) * u64::from(seq.max_frame_height)
                    };
                    let record = FrameRecord {
                        packet: packet_index,
                        show_existing_frame: header.show_existing_frame,
                        show_frame: header.show_frame,
                        frame_type: header.frame_type,
                        frame_to_show_map_idx: header.frame_to_show_map_idx.unwrap_or(0),
                        refresh_frame_flags: header.refresh_frame_flags.unwrap_or(0),
                        luma_samples,
                        buffer_removal_time: header.buffer_removal_time.first().copied().flatten(),
                        frame_presentation_time: header.frame_presentation_time,
                        coded_bits: 0,
                    };
                    if header.show_existing_frame {
                        records.push(record);
                    } else {
                        open = Some(OpenFrame {
                            record,
                            bits_through_last_obu: pending_bits,
                        });
                    }
                }
                ObuType::TileGroup => {
                    pending_bits += bits;
                    if let Some(frame) = open.as_mut() {
                        frame.bits_through_last_obu = pending_bits;
                    }
                }
                _ => pending_bits += bits,
            }
        }
    }
    close(&mut open, &mut pending_bits, &mut records);
    let seq = seq.ok_or_else(|| "no sequence header".to_string())?;
    Ok((seq, records))
}
