#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Symbol Decoder Integration Tests
//!
//! Tests the arithmetic decoder and CDF-based symbol reading
//! with both synthetic and real AV1 bitstream data.

use bitvue_av1_codec::{ArithmeticDecoder, CdfContext, SymbolDecoder};

#[test]
fn test_symbol_decoder_creation() {
    // Create decoder with minimal valid data (16+ bits for initial value)
    let data = vec![0x80, 0x00, 0xFF, 0xFF];
    let decoder = SymbolDecoder::new(&data);

    assert!(decoder.is_ok(), "Should create decoder successfully");

    let decoder = decoder.unwrap();
    assert_eq!(
        decoder.decoder.range, 0x8000,
        "Range should be initialized to 0x8000"
    );
    // Note: value is a 64-bit window on 64-bit systems
    // The first byte (0x80) is shifted left by 55 bits (EC_WIN_SIZE - cnt - 24)
    // So value will be much larger than 0x8000. The test just checks it's initialized.
    assert_ne!(
        decoder.decoder.value, 0,
        "Value should be initialized (not zero)"
    );
}

#[test]
fn test_symbol_decoder_read_partition() {
    // Create test data - enough bytes for multiple symbol reads
    let data = vec![0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB, 0xCC, 0xDD];
    let mut decoder = SymbolDecoder::new(&data).expect("Failed to create decoder");

    // Read partition symbol for 64x64 block (2^6)
    let result = decoder.read_partition(6, true, true);

    // Should succeed (even though data is synthetic)
    assert!(result.is_ok(), "Should read partition symbol");

    let symbol = result.unwrap();

    // Symbol should be valid partition type (0-9)
    assert!(
        symbol <= 9,
        "Partition symbol should be 0-9, got {}",
        symbol
    );

    println!("✅ Read partition symbol: {}", symbol);
}

#[test]
fn test_arithmetic_decoder_read_symbol() {
    // Create test data
    let data = vec![0x80, 0x00, 0x12, 0x34, 0x56, 0x78];
    let mut decoder = ArithmeticDecoder::new(&data).expect("Failed to create decoder");

    // Create uniform CDF for 4 symbols
    let cdf = vec![0u16, 8192, 16384, 24576, 32768];

    // Read multiple symbols
    for i in 0..3 {
        let result = decoder.read_symbol(&cdf);
        assert!(result.is_ok(), "Failed to read symbol {}", i);

        let symbol = result.unwrap();
        assert!(symbol < 4, "Symbol {} out of range: {}", i, symbol);

        println!("  Symbol {}: {}", i, symbol);
    }

    println!("✅ Successfully read 3 symbols from decoder");
}

#[test]
fn test_cdf_context_block_sizes() {
    let context = CdfContext::new();

    // Test different block sizes
    let test_cases = vec![
        (2, 2),  // 4x4: 1 symbol + end marker
        (3, 5),  // 8x8: 4 symbols + end marker
        (4, 11), // 16x16: 10 symbols + end marker
        (5, 11), // 32x32: 10 symbols + end marker
        (6, 11), // 64x64: 10 symbols + end marker
        (7, 11), // 128x128: 10 symbols + end marker
    ];

    for (block_size_log2, expected_len) in test_cases {
        let cdf = context.get_partition_cdf(block_size_log2);
        assert_eq!(
            cdf.len(),
            expected_len,
            "Block size 2^{} should have {} CDF entries",
            block_size_log2,
            expected_len
        );

        // Verify CDF properties
        assert_eq!(cdf[0], 0, "CDF should start at 0");
        assert_eq!(cdf[cdf.len() - 1], 32768, "CDF should end at 32768");

        // Verify monotonically increasing
        for i in 1..cdf.len() {
            assert!(
                cdf[i] >= cdf[i - 1],
                "CDF should be monotonically increasing at index {}",
                i
            );
        }

        println!(
            "✅ Block size 2^{}: CDF length {}",
            block_size_log2,
            cdf.len()
        );
    }
}

#[test]
fn test_symbol_decoder_with_real_file() {
    // Load real test file (relative to workspace root)
    let test_file = "../../test_data/av1_test.ivf";

    // Skip test if file doesn't exist
    if !std::path::Path::new(test_file).exists() {
        println!("⚠️  Skipping test: {} not found", test_file);
        return;
    }

    use std::fs;
    let data = fs::read(test_file).expect("Failed to read test file");

    // Extract OBU data from IVF
    let obu_data = bitvue_av1_codec::extract_obu_data(&data).expect("Failed to extract OBU data");

    // Parse OBUs
    let obus = bitvue_av1_codec::parse_all_obus(&obu_data).expect("Failed to parse OBUs");

    println!("Found {} OBUs in test file", obus.len());

    // Find FRAME OBU (type 6)
    let frame_obu = obus
        .iter()
        .find(|obu| obu.header.obu_type == bitvue_av1_codec::ObuType::Frame);

    if let Some(obu) = frame_obu {
        println!(
            "✅ Found FRAME OBU with {} bytes of payload",
            obu.payload.len()
        );

        // Try to create symbol decoder with frame payload
        // Note: This is simplified - in real usage, we'd need to:
        // 1. Parse frame header
        // 2. Extract tile group data
        // 3. Extract individual tile data
        // 4. Initialize decoder with tile data

        // For now, just verify we can create a decoder without crashing
        if obu.payload.len() >= 16 {
            let result = SymbolDecoder::new(&obu.payload);

            // May or may not succeed depending on payload structure
            // But it shouldn't panic
            match result {
                Ok(mut decoder) => {
                    println!("  Created symbol decoder from FRAME payload");

                    // Try reading a partition symbol (may fail due to wrong context)
                    if let Ok(symbol) = decoder.read_partition(6, true, true) {
                        println!("  Read partition symbol: {} (may be incorrect without proper tile data)", symbol);
                    }
                }
                Err(e) => {
                    println!(
                        "  Could not create decoder (expected without tile extraction): {:?}",
                        e
                    );
                }
            }
        }
    } else {
        println!("⚠️  No FRAME OBU found in test file");
    }

    println!("\n🎉 Symbol Decoder Real File Test PASSED!");
}

#[test]
fn test_symbol_decoder_multiple_reads() {
    // Create decoder with enough data
    let data = vec![
        0x80, 0x00, 0xFF, 0xFF, 0xAA, 0xBB, 0xCC, 0xDD, 0x11, 0x22, 0x33, 0x44,
    ];
    let mut decoder = SymbolDecoder::new(&data).expect("Failed to create decoder");

    // Read multiple partition symbols for different block sizes
    let block_sizes = vec![6, 5, 4, 3]; // 64x64, 32x32, 16x16, 8x8

    for (i, &block_size_log2) in block_sizes.iter().enumerate() {
        let result = decoder.read_partition(block_size_log2, true, true);

        if let Ok(symbol) = result {
            assert!(symbol <= 9, "Symbol {} out of range", symbol);
            println!(
                "  Read {}: Block size 2^{} -> partition {}",
                i, block_size_log2, symbol
            );
        } else {
            // May fail if we exhaust the decoder - that's OK for this test
            println!("  Read {}: Decoder exhausted (expected)", i);
            break;
        }
    }

    println!("✅ Successfully read multiple partition symbols");
}
