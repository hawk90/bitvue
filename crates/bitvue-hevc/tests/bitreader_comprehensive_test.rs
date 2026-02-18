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
//! Comprehensive tests for HEVC bitreader module

use bitvue_hevc::bitreader::{remove_emulation_prevention_bytes, BitReader};

#[test]
fn test_read_bit_single_byte() {
    let data = vec![0b10110010];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_bit().unwrap(), true);
}

#[test]
fn test_read_bits_cross_byte() {
    let data = vec![0xAA, 0x55];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_bits(12).unwrap(), 0xAA5);
}

#[test]
fn test_read_byte_basic() {
    let data = vec![0xAB, 0xCD];
    let mut reader = BitReader::new(&data);
    assert_eq!(reader.read_byte().unwrap(), 0xAB);
}

#[test]
fn test_remove_emulation_prevention() {
    let data = vec![0x00, 0x00, 0x03, 0x01];
    let result = remove_emulation_prevention_bytes(&data);
    assert_eq!(result, vec![0x00, 0x00, 0x01]);
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
