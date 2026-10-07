//! Human-readable output for `decode`: frame table and stream statistics.

use super::extract::FrameRecord;

// ─── Frame table output ───────────────────────────────────────────────────────

pub(super) fn print_frame_table(records: &[FrameRecord], show_md5: bool) {
    if show_md5 {
        println!(
            "{:<6} {:<8} {:<10} {:<16} {:<12} {:<6} MD5",
            "Index", "Type", "Size", "PTS", "Offset", "Key"
        );
        println!("{}", "-".repeat(85));
    } else {
        println!(
            "{:<6} {:<8} {:<10} {:<16} {:<12} Key",
            "Index", "Type", "Size", "PTS", "Offset"
        );
        println!("{}", "-".repeat(66));
    }

    for r in records {
        let pts_str = r
            .pts
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        if show_md5 {
            println!(
                "{:<6} {:<8} {:<10} {:<16} {:<12} {:<6} {}",
                r.index,
                r.frame_type,
                r.size,
                pts_str,
                r.offset,
                if r.key_frame { "Y" } else { "" },
                r.md5_hex.as_deref().unwrap_or("-"),
            );
        } else {
            println!(
                "{:<6} {:<8} {:<10} {:<16} {:<12} {}",
                r.index,
                r.frame_type,
                r.size,
                pts_str,
                r.offset,
                if r.key_frame { "Y" } else { "" },
            );
        }
    }

    println!("\nTotal: {} frame(s)", records.len());
}

// ─── Statistics ───────────────────────────────────────────────────────────────

pub(super) fn print_stream_stats(records: &[FrameRecord], detailed: bool) {
    if records.is_empty() {
        println!("Statistics: no frames");
        return;
    }

    // Frame type counts
    let mut type_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut key_count = 0usize;
    let mut total_bytes = 0usize;

    for r in records {
        *type_counts.entry(r.frame_type.clone()).or_insert(0) += 1;
        if r.key_frame {
            key_count += 1;
        }
        total_bytes += r.size;
    }

    let avg_bytes = total_bytes / records.len();
    let max_frame = records.iter().max_by_key(|r| r.size).unwrap();
    let min_frame = records.iter().min_by_key(|r| r.size).unwrap();

    println!("── Stream Statistics ──────────────────────────────");
    println!("  Frames:     {}", records.len());
    println!("  Key frames: {}", key_count);
    println!(
        "  Total size: {} bytes ({:.2} MB)",
        total_bytes,
        total_bytes as f64 / 1_048_576.0
    );
    println!(
        "  Avg size:   {} bytes ({:.2} KB)",
        avg_bytes,
        avg_bytes as f64 / 1024.0
    );
    println!(
        "  Max frame:  #{} {} bytes",
        max_frame.index, max_frame.size
    );
    println!(
        "  Min frame:  #{} {} bytes",
        min_frame.index, min_frame.size
    );

    println!("  Frame type distribution:");
    for (t, n) in &type_counts {
        println!(
            "    {:8} {:>5}  ({:.1}%)",
            t,
            n,
            (*n as f64 / records.len() as f64) * 100.0
        );
    }

    if detailed {
        // Size buckets (logarithmic)
        println!("  Size distribution:");
        let buckets = [1024, 4096, 16384, 65536, usize::MAX];
        let labels = ["<1KB", "<4KB", "<16KB", "<64KB", "≥64KB"];
        let mut counts = [0usize; 5];
        for r in records {
            for (i, &threshold) in buckets.iter().enumerate() {
                if r.size < threshold {
                    counts[i] += 1;
                    break;
                }
            }
        }
        for (label, count) in labels.iter().zip(counts.iter()) {
            if *count > 0 {
                println!(
                    "    {:8} {:>5}  ({:.1}%)",
                    label,
                    count,
                    (*count as f64 / records.len() as f64) * 100.0
                );
            }
        }
    }
}

// ─── HEVC-specific stream statistics ─────────────────────────────────────────

/// Print HEVC NAL unit type distribution.
/// Re-parses the bitstream so it is only called when --stream-stats is requested.
pub(super) fn print_hevc_nal_stats(data: &[u8]) {
    use bitvue_hevc::parse_hevc;
    let stream = match parse_hevc(data) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("  (HEVC NAL stats unavailable: {})", e);
            return;
        }
    };

    // NAL unit type distribution
    let mut nal_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut total_nal_bytes = 0usize;
    for nal in &stream.nal_units {
        let label = format!("{:?}", nal.header.nal_unit_type);
        *nal_counts.entry(label).or_insert(0) += 1;
        total_nal_bytes += nal.size as usize;
    }

    println!("  HEVC NAL unit distribution:");
    for (label, count) in &nal_counts {
        println!(
            "    {:30} {:>5}  ({:.1}%)",
            label,
            count,
            (*count as f64 / stream.nal_units.len().max(1) as f64) * 100.0
        );
    }
    println!(
        "  Total NAL units: {}  ({:.2} MB payload)",
        stream.nal_units.len(),
        total_nal_bytes as f64 / 1_048_576.0
    );

    // Slice type distribution
    if !stream.slices.is_empty() {
        let mut slice_counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for slice in &stream.slices {
            let nal = &stream.nal_units[slice.nal_index];
            let t = if nal.header.nal_unit_type.is_idr() {
                "IDR".to_string()
            } else if nal.header.nal_unit_type.is_irap() {
                "IRAP".to_string()
            } else {
                format!("{:?}", nal.header.nal_unit_type)
            };
            *slice_counts.entry(t).or_insert(0) += 1;
        }
        println!("  HEVC slice type distribution:");
        for (t, n) in &slice_counts {
            println!(
                "    {:30} {:>5}  ({:.1}%)",
                t,
                n,
                (*n as f64 / stream.slices.len().max(1) as f64) * 100.0
            );
        }
    }
}
