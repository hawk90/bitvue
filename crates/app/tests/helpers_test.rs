//! Tests for Helper Functions

#[test]
fn test_frame_index_helpers() {
    // Test frame index manipulation helpers
    fn clamp_frame_index(index: usize, max_frames: usize) -> usize {
        index.min(max_frames.saturating_sub(1))
    }

    assert_eq!(clamp_frame_index(50, 100), 50);
    assert_eq!(clamp_frame_index(150, 100), 99);
}

#[test]
fn test_time_formatting_helpers() {
    // Test time formatting helpers
    fn format_milliseconds(ms: u64) -> String {
        let seconds = ms / 1000;
        let minutes = seconds / 60;
        let hours = minutes / 60;

        if hours > 0 {
            format!("{}:{:02}:{:02}", hours, minutes % 60, seconds % 60)
        } else if minutes > 0 {
            format!("{}:{:02}", minutes, seconds % 60)
        } else {
            format!("{}s", seconds)
        }
    }

    assert_eq!(format_milliseconds(5000), "5s");
    assert_eq!(format_milliseconds(65000), "1:05");
}

#[test]
fn test_color_conversion_helpers() {
    // Test color conversion helpers
    fn rgb_to_hex(r: u8, g: u8, b: u8) -> String {
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }

    assert_eq!(rgb_to_hex(255, 0, 0), "#FF0000");
    assert_eq!(rgb_to_hex(0, 255, 0), "#00FF00");
}

#[test]
fn test_string_truncation_helpers() {
    // Test string truncation helpers
    fn truncate_string(s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }

    assert_eq!(truncate_string("short", 10), "short");
    assert_eq!(truncate_string("very long string", 10), "very lo...");
}

#[test]
fn test_percentage_calculation_helpers() {
    // Test percentage calculation helpers
    fn calculate_percentage(part: usize, total: usize) -> f64 {
        if total == 0 {
            0.0
        } else {
            (part as f64 / total as f64) * 100.0
        }
    }

    assert_eq!(calculate_percentage(50, 100), 50.0);
    assert_eq!(calculate_percentage(0, 0), 0.0);
}

#[test]
fn test_range_helpers() {
    // Test range checking helpers
    fn in_range<T: PartialOrd>(value: T, min: T, max: T) -> bool {
        value >= min && value <= max
    }

    assert!(in_range(5, 0, 10));
    assert!(!in_range(15, 0, 10));
}

#[test]
fn test_lerp_helpers() {
    // Test linear interpolation helpers
    fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t
    }

    assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
    assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
    assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
}

#[test]
fn test_debounce_helpers() {
    // Test debounce helper
    struct Debounce {
        last_call_time_ms: u64,
        delay_ms: u64,
    }

    impl Debounce {
        fn should_execute(&self, current_time_ms: u64) -> bool {
            current_time_ms - self.last_call_time_ms >= self.delay_ms
        }
    }

    let debounce = Debounce {
        last_call_time_ms: 1000,
        delay_ms: 100,
    };

    assert!(debounce.should_execute(1150));
    assert!(!debounce.should_execute(1050));
}

#[test]
fn test_array_helpers() {
    // Test array manipulation helpers
    fn rotate_array<T: Clone>(arr: &[T], positions: usize) -> Vec<T> {
        let len = arr.len();
        if len == 0 {
            return vec![];
        }

        let positions = positions % len;
        let mut result = Vec::with_capacity(len);
        result.extend_from_slice(&arr[positions..]);
        result.extend_from_slice(&arr[..positions]);
        result
    }

    let arr = vec![1, 2, 3, 4, 5];
    let rotated = rotate_array(&arr, 2);

    assert_eq!(rotated, vec![3, 4, 5, 1, 2]);
}

#[test]
fn test_averaging_helpers() {
    // Test averaging helpers
    fn moving_average(values: &[f64], window: usize) -> Vec<f64> {
        values
            .windows(window)
            .map(|w| w.iter().sum::<f64>() / window as f64)
            .collect()
    }

    let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let avg = moving_average(&values, 3);

    assert_eq!(avg.len(), 3);
    assert_eq!(avg[0], 2.0); // (1+2+3)/3
}
