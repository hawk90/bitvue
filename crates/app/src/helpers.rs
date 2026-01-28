//! Helper functions for bitstream analysis and navigation
//!
//! Utility functions for finding units, frames, and formatting data.

use bitvue_core::selection::SelectionState;
use bitvue_core::{UnitKey, UnitNode};

/// Find a unit by its key (recursive search)
pub fn find_unit_by_key<'a>(units: &'a [UnitNode], key: &UnitKey) -> Option<&'a UnitNode> {
    for unit in units {
        if unit.key == *key {
            return Some(unit);
        }
        if !unit.children.is_empty() {
            if let Some(found) = find_unit_by_key(&unit.children, key) {
                return Some(found);
            }
        }
    }
    None
}

/// Find a unit by its key with index (helper for syntax parsing with bit-level tracking)
pub fn find_unit_by_key_with_index<'a>(
    units: &'a [UnitNode],
    key: &UnitKey,
) -> Option<(usize, &'a UnitNode)> {
    fn find_recursive<'a>(
        units: &'a [UnitNode],
        key: &UnitKey,
        index: &mut usize,
    ) -> Option<(usize, &'a UnitNode)> {
        for unit in units {
            let current_index = *index;
            *index += 1;

            if unit.key == *key {
                return Some((current_index, unit));
            }
            if !unit.children.is_empty() {
                if let Some(found) = find_recursive(&unit.children, key, index) {
                    return Some(found);
                }
            }
        }
        None
    }

    let mut index = 0;
    find_recursive(units, key, &mut index)
}

/// Find unit containing the given byte offset (TC06 helper)
pub fn find_unit_containing_offset_helper(units: &[UnitNode], offset: u64) -> Option<UnitNode> {
    for unit in units {
        if offset >= unit.offset && offset < unit.offset + unit.size as u64 {
            return Some(unit.clone());
        }
        if !unit.children.is_empty() {
            if let Some(found) = find_unit_containing_offset_helper(&unit.children, offset) {
                return Some(found);
            }
        }
    }
    None
}

/// Format bytes with appropriate unit (B, KB, MB, GB)
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Get current process memory usage in MB
pub fn get_memory_usage_mb() -> u64 {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("ps")
            .args(&["-o", "rss=", "-p", &std::process::id().to_string()])
            .output();

        if let Ok(output) = output {
            let s = String::from_utf8_lossy(&output.stdout);
            if let Ok(kb) = s.trim().parse::<u64>() {
                return kb / 1024; // KB to MB
            }
        }
        0
    }

    #[cfg(target_os = "linux")]
    {
        use std::fs;
        if let Ok(status) = fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    if let Some(kb_str) = line.split_whitespace().nth(1) {
                        if let Ok(kb) = kb_str.parse::<u64>() {
                            return kb / 1024; // KB to MB
                        }
                    }
                }
            }
        }
        0
    }

    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::ProcessStatus::{
            GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
        };
        use windows::Win32::System::Threading::GetCurrentProcess;

        unsafe {
            let mut pmc: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
            pmc.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;

            if GetProcessMemoryInfo(GetCurrentProcess(), &mut pmc, pmc.cb).is_ok() {
                return (pmc.WorkingSetSize as u64) / (1024 * 1024); // Bytes to MB
            }
        }
        0
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        0 // Unsupported platform
    }
}

/// Find the first frame unit in the tree
pub fn find_first_frame(units: &[UnitNode]) -> Option<&UnitNode> {
    for unit in units {
        if unit.frame_index.is_some() {
            return Some(unit);
        }
        if !unit.children.is_empty() {
            if let Some(found) = find_first_frame(&unit.children) {
                return Some(found);
            }
        }
    }
    None
}

/// Count total number of frames in the unit tree
pub fn count_frames(units: &[UnitNode]) -> usize {
    let mut count = 0;
    count_frames_recursive(units, &mut count);
    count
}

fn count_frames_recursive(units: &[UnitNode], count: &mut usize) {
    for unit in units {
        if unit.frame_index.is_some() {
            *count += 1;
        }
        if !unit.children.is_empty() {
            count_frames_recursive(&unit.children, count);
        }
    }
}

/// Find frame by index (0-based)
pub fn find_frame_by_index(units: &[UnitNode], target_index: usize) -> Option<&UnitNode> {
    let mut current_index = 0;
    find_frame_by_index_recursive(units, target_index, &mut current_index)
}

fn find_frame_by_index_recursive<'a>(
    units: &'a [UnitNode],
    target_index: usize,
    current_index: &mut usize,
) -> Option<&'a UnitNode> {
    for unit in units {
        if unit.frame_index.is_some() {
            if *current_index == target_index {
                return Some(unit);
            }
            *current_index += 1;
        }
        if !unit.children.is_empty() {
            if let Some(found) =
                find_frame_by_index_recursive(&unit.children, target_index, current_index)
            {
                return Some(found);
            }
        }
    }
    None
}

/// Get current frame index from selection
pub fn get_current_frame_index(selection: &SelectionState, units: &[UnitNode]) -> Option<usize> {
    let unit_key = selection.unit.as_ref()?;
    let mut current_index = 0;
    get_frame_index_recursive(units, unit_key, &mut current_index)
}

fn get_frame_index_recursive(
    units: &[UnitNode],
    target_key: &UnitKey,
    current_index: &mut usize,
) -> Option<usize> {
    for unit in units {
        if unit.frame_index.is_some() {
            if unit.key == *target_key {
                return Some(*current_index);
            }
            *current_index += 1;
        }
        if !unit.children.is_empty() {
            if let Some(index) =
                get_frame_index_recursive(&unit.children, target_key, current_index)
            {
                return Some(index);
            }
        }
    }
    None
}
