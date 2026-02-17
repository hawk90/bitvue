#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_comparisons)]
#![allow(unused_doc_comments)]
//! Tests for partition tree data and visualization

use bitvue_core::partition_grid::{PartitionBlock, PartitionGrid, PartitionKind, PartitionType};

#[test]
fn test_partition_kind_color() {
    assert_eq!(PartitionKind::Intra.tint_color(), (100, 150, 255));
    assert_eq!(PartitionKind::Inter.tint_color(), (255, 150, 100));
}

#[test]
fn test_partition_block_contains() {
    let block = PartitionBlock::new(10, 20, 30, 40, PartitionType::None, 0);

    assert!(block.contains(10, 20)); // Top-left corner
    assert!(block.contains(39, 59)); // Bottom-right (exclusive)
    assert!(!block.contains(40, 60)); // Outside
    assert!(!block.contains(5, 20)); // Left of block
}

#[test]
fn test_partition_block_area() {
    let block = PartitionBlock::new(0, 0, 64, 64, PartitionType::None, 0);
    assert_eq!(block.area(), 4096);
}

#[test]
fn test_partition_grid_creation() {
    let mut grid = PartitionGrid::new(1920, 1080, 64);

    grid.add_block(PartitionBlock::new(0, 0, 64, 64, PartitionType::None, 0));
    grid.add_block(PartitionBlock::new(64, 0, 32, 32, PartitionType::Split, 1));

    assert_eq!(grid.block_count(), 2);
}

#[test]
fn test_partition_grid_block_at() {
    let mut grid = PartitionGrid::new(128, 128, 64);

    grid.add_block(PartitionBlock::new(0, 0, 64, 64, PartitionType::None, 0));
    grid.add_block(PartitionBlock::new(64, 0, 64, 64, PartitionType::Split, 0));

    let block = grid.block_at(32, 32).unwrap();
    assert_eq!(block.partition, PartitionType::None);

    let block = grid.block_at(96, 32).unwrap();
    assert_eq!(block.partition, PartitionType::Split);

    assert!(grid.block_at(200, 200).is_none());
}

#[test]
fn test_create_scaffold() {
    let grid = PartitionGrid::create_scaffold(1920, 1080, 64);

    // 1920/64 = 30, 1080/64 = 16.875 -> 17
    let expected_cols = (1920 + 63) / 64; // 30
    let expected_rows = (1080 + 63) / 64; // 17

    assert_eq!(grid.block_count(), (expected_cols * expected_rows) as usize);
    assert_eq!(grid.sb_size, 64);
}

#[test]
fn test_partition_statistics() {
    let mut grid = PartitionGrid::new(128, 128, 64);

    // Add blocks at different depths
    grid.add_block(PartitionBlock::new(0, 0, 64, 64, PartitionType::None, 0));
    grid.add_block(PartitionBlock::new(64, 0, 32, 32, PartitionType::Split, 1));
    grid.add_block(PartitionBlock::new(96, 0, 32, 32, PartitionType::Split, 1));
    grid.add_block(PartitionBlock::new(64, 32, 32, 32, PartitionType::Horz, 1));
    grid.add_block(PartitionBlock::new(96, 32, 32, 32, PartitionType::Vert, 1));

    let stats = grid.statistics();

    assert_eq!(stats.total_blocks, 5);
    assert_eq!(stats.none_count, 1);
    assert_eq!(stats.split_count, 2);
    assert_eq!(stats.horz_count, 1);
    assert_eq!(stats.vert_count, 1);
    assert_eq!(stats.max_depth(), 1);
    assert!(stats.avg_depth() > 0.5);
}

#[test]
fn test_blocks_in_viewport() {
    let mut grid = PartitionGrid::new(256, 256, 64);

    grid.add_block(PartitionBlock::new(0, 0, 64, 64, PartitionType::None, 0));
    grid.add_block(PartitionBlock::new(64, 0, 64, 64, PartitionType::None, 0));
    grid.add_block(PartitionBlock::new(128, 0, 64, 64, PartitionType::None, 0));
    grid.add_block(PartitionBlock::new(192, 0, 64, 64, PartitionType::None, 0));

    // Viewport that covers first two blocks
    let visible = grid.blocks_in_viewport(0, 0, 100, 100);
    assert_eq!(visible.len(), 2);

    // Viewport that covers nothing
    let visible = grid.blocks_in_viewport(300, 300, 100, 100);
    assert_eq!(visible.len(), 0);
}

#[test]
fn test_cache_key() {
    let grid = PartitionGrid::create_scaffold(1920, 1080, 64);
    let key = grid.cache_key("stream_A", 42);

    assert!(key.contains("partition:stream_A:f42"));
    assert!(key.contains("1920x1080"));
    assert!(key.contains("sb64"));
}
