use bitvue_formats::mp4::{extract_avc_samples, parse_mp4};

const SAMPLE_FILE: &str = "/Users/hawk/Workspaces/bitvue/samples/foreman_h264.mp4";

#[test]
fn test_real_h264_mp4() {
    let data = std::fs::read(SAMPLE_FILE).expect("Failed to read sample file");

    println!("File size: {} bytes", data.len());

    // Parse MP4 structure
    let info = parse_mp4(&data).expect("Failed to parse MP4");
    println!("MP4 Info:");
    println!("  Brand: {:?}", info.brand);
    println!("  Codec: {:?}", info.codec);
    println!("  Sample offsets: {} entries", info.sample_offsets.len());
    println!("  Sample sizes: {} entries", info.sample_sizes.len());

    if !info.sample_offsets.is_empty() {
        println!(
            "  First 5 offsets: {:?}",
            &info.sample_offsets[..5.min(info.sample_offsets.len())]
        );
    }
    if !info.sample_sizes.is_empty() {
        println!(
            "  First 5 sizes: {:?}",
            &info.sample_sizes[..5.min(info.sample_sizes.len())]
        );
    }

    // Extract samples
    match extract_avc_samples(&data) {
        Ok(samples) => {
            println!("\nExtracted {} samples", samples.len());
            assert!(samples.len() > 0, "Should extract at least one sample");
        }
        Err(e) => {
            println!("\nExtraction error: {:?}", e);
            panic!("Failed to extract AVC samples: {:?}", e);
        }
    }
}
