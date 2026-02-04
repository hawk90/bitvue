//! Profile data recording utilities.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::time::Duration;

/// Recorded profile data.
#[derive(Clone, Debug)]
pub struct ProfileData {
    pub label: String,
    pub duration: Duration,
    pub metadata: BTreeMap<String, String>,
}

impl ProfileData {
    pub fn new(label: impl Into<String>, duration: Duration) -> Self {
        Self {
            label: label.into(),
            duration,
            metadata: BTreeMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Records profile data.
pub struct ProfileRecorder {
    data: Vec<ProfileData>,
    enabled: bool,
}

impl ProfileRecorder {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            enabled: true,
        }
    }

    pub fn record(&mut self, profile: ProfileData) {
        if self.enabled {
            self.data.push(profile);
        }
    }

    pub fn record_simple(&mut self, label: impl Into<String>, duration: Duration) {
        self.record(ProfileData::new(label, duration));
    }

    pub fn data(&self) -> &[ProfileData] {
        &self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Filters data by label prefix.
    pub fn filter_by_prefix(&self, prefix: &str) -> Vec<&ProfileData> {
        self.data
            .iter()
            .filter(|d| d.label.starts_with(prefix))
            .collect()
    }

    /// Gets total duration for all records.
    pub fn total_duration(&self) -> Duration {
        self.data.iter().map(|d| d.duration).sum()
    }
}

impl Default for ProfileRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile_recorder() {
        let mut recorder = ProfileRecorder::new();
        recorder.record_simple("test", Duration::from_millis(100));

        assert_eq!(recorder.len(), 1);
        assert!(!recorder.is_empty());
    }

    #[test]
    fn test_profile_recorder_clear() {
        let mut recorder = ProfileRecorder::new();
        recorder.record_simple("test", Duration::from_millis(100));
        recorder.clear();
        assert!(recorder.is_empty());
    }

    #[test]
    fn test_profile_recorder_disabled() {
        let mut recorder = ProfileRecorder::new();
        recorder.set_enabled(false);
        recorder.record_simple("test", Duration::from_millis(100));
        assert!(recorder.is_empty());
    }

    #[test]
    fn test_profile_recorder_filter() {
        let mut recorder = ProfileRecorder::new();
        recorder.record_simple("foo::bar", Duration::from_millis(100));
        recorder.record_simple("foo::baz", Duration::from_millis(200));
        recorder.record_simple("qux", Duration::from_millis(300));

        let filtered = recorder.filter_by_prefix("foo::");
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn test_total_duration() {
        let mut recorder = ProfileRecorder::new();
        recorder.record_simple("test1", Duration::from_millis(100));
        recorder.record_simple("test2", Duration::from_millis(200));

        assert_eq!(recorder.total_duration(), Duration::from_millis(300));
    }
}
