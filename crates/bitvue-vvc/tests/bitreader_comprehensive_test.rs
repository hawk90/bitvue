#![allow(hidden_glob_reexports)]
#![allow(unreachable_code)]
#![allow(non_camel_case_types)]
#![allow(unused_assignments)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Comprehensive tests for VVC bitreader module
//!
//! Tests BitReader, Exp-Golomb coding, emulation prevention

use bitvue_vvc::bitreader::{remove_emulation_prevention_bytes, BitReader};

// Basic reading tests
#[test]
fn test_read_bit_single_byte() {
    let data = vec![0b10110100];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_bit().unwrap(), true);
    assert_eq!(reader.read_bit().unwrap(), false);
    assert_eq!(reader.read_bit().unwrap(), true);
}

#[test]
fn test_read_bits_single_byte() {
    let data = vec![0b10110010];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_bits(4).unwrap(), 0b1011);
    assert_eq!(reader.read_bits(4).unwrap(), 0b0010);
}

#[test]
fn test_read_byte_basic() {
    let data = vec![0xAB, 0xCD];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_byte().unwrap(), 0xAB);
    assert_eq!(reader.read_byte().unwrap(), 0xCD);
}

#[test]
fn test_skip_bits_basic() {
    let data = vec![0xFF, 0xAA];
    let mut reader = BitReader::new(&data);
    reader.skip_bits(8).unwrap();
    assert_eq!(reader.position(), 8);
    assert_eq!(reader.read_bits(8).unwrap(), 0xAA);
}

#[test]
fn test_byte_align_mid_byte() {
    let data = vec![0b10101010, 0xFF];
    let mut reader = BitReader::new(&data);
    reader.read_bit().unwrap();
    reader.byte_align();
    assert_eq!(reader.position(), 8);
    assert!(reader.is_byte_aligned());
}

#[test]
fn test_remove_emulation_prevention_single() {
    let data = vec![0x00, 0x00, 0x03, 0x01];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00, 0x01]);
}

#[test]
fn test_remove_emulation_prevention_none() {
    let data = vec![0x00, 0x00, 0x01, 0x00];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00, 0x01, 0x00]);
}

#[test]
fn test_remove_emulation_prevention_multiple() {
    let data = vec![
        0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x03, 0x02, 0x00, 0x00, 0x03, 0x03,
    ];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(
        result,
        vec![0x00, 0x00, 0x01, 0x00, 0x00, 0x02, 0x00, 0x00, 0x03,]
    );
}

#[test]
fn test_read_ue_zero() {
    let data = vec![0b10000000];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_ue().unwrap(), 0);
}

#[test]
fn test_read_se_zero() {
    let data = vec![0b10000000];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_se().unwrap(), 0);
}

#[test]
fn test_read_rbsp_trailing_bits_valid() {
    let data = vec![0b10000000];
    let mut reader = BitReader::new(&data);
    assert!(reader.read_rbsp_trailing_bits().is_ok());
    assert!(reader.is_byte_aligned());
}

#[test]
fn test_read_rbsp_trailing_bits_invalid_stop() {
    let data = vec![0b00000000];
    let mut reader = BitReader::new(&data);
    assert!(reader.read_rbsp_trailing_bits().is_err());
}

#[test]
fn test_more_rbsp_data_true() {
    let data = vec![0xFF];
    let reader = BitReader::new(&data);
    assert!(reader.more_rbsp_data());
}

#[test]
fn test_more_rbsp_data_empty() {
    let data: &[u8] = &[];
    let reader = BitReader::new(&data);
    assert!(!reader.more_rbsp_data());
}

#[test]
fn test_peek_bits_basic() {
    let data = vec![0b10110100];
    let reader = BitReader::new(&data);
    assert_eq!(reader.peek_bits(4).unwrap(), 0b1011);
    assert_eq!(reader.position(), 0);
}

#[test]
fn test_peek_bits_eof() {
    let data = vec![0x00];
    let reader = BitReader::new(&data);
    assert!(reader.peek_bits(9).is_err());
}

#[test]
fn test_read_u_basic() {
    let data = vec![0b10101010];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_u(4).unwrap(), 0b1010);
    assert_eq!(reader.read_u(4).unwrap(), 0b1010);
}

#[test]
fn test_read_f_basic() {
    let data = vec![0b11001100];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_f(8).unwrap(), 0b11001100);
}
