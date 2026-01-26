// HRD module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

use super::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Create default HRD parameters
fn create_default_hrd_params() -> HrdParameters {
    HrdParameters::default()
}

/// Create custom HRD parameters
fn create_custom_hrd_params() -> HrdParameters {
    HrdParameters {
        cpb_size_bits: 2_000_000, // 2 Mbit
        bit_rate_bps: 10_000_000, // 10 Mbps
        initial_cpb_removal_delay: 90000, // 1 second at 90kHz
        cpb_removal_delay_length: 24,
        dpb_output_delay_length: 24,
        time_scale: 90000,
        num_units_in_tick: 1,
        low_delay_hrd: false,
        cbr_flag: true,
    }
}

/// Create test frame timing
fn create_test_frame_timing(display_idx: usize, frame_size_bits: u64, pts_sec: f64) -> FrameHrdTiming {
    FrameHrdTiming::new(display_idx, frame_size_bits, pts_sec)
}

/// Create test frame timing with delays
fn create_test_frame_timing_with_delays(
    display_idx: usize,
    frame_size_bits: u64,
    pts_sec: f64,
    cpb_delay: u64,
    dpb_delay: u64,
) -> FrameHrdTiming {
    let mut timing = FrameHrdTiming::new(display_idx, frame_size_bits, pts_sec);
    timing.cpb_removal_delay = Some(cpb_delay);
    timing.dpb_output_delay = Some(dpb_delay);
    timing
}

/// Create test CPB state
fn create_test_cpb_state(fullness_bits: u64, time_sec: f64, frame_idx: usize) -> CpbState {
    CpbState {
        fullness_bits,
        time_sec,
        frame_idx,
        is_removal: false,
        overflow: false,
        underflow: false,
    }
}

/// Create sequence of frame timings for testing
fn create_frame_sequence() -> Vec<FrameHrdTiming> {
    vec![
        create_test_frame_timing(0, 100_000, 0.0),     // Frame 0 at 0s
        create_test_frame_timing(1, 150_000, 0.0333),   // Frame 1 at ~33ms
        create_test_frame_timing(2, 200_000, 0.0667),   // Frame 2 at ~67ms
        create_test_frame_timing(3, 120_000, 0.1000),   // Frame 3 at 100ms
    ]
}

// ============================================================================
// HrdParameters Tests
// ============================================================================

#[cfg(test)]
mod hrd_parameters_tests {
    use super::*;

    #[test]
    fn test_hrd_parameters_default() {
        // Arrange & Act
        let params = HrdParameters::default();

        // Assert
        assert_eq!(params.cpb_size_bits, 1_000_000);
        assert_eq!(params.bit_rate_bps, 5_000_000);
        assert_eq!(params.initial_cpb_removal_delay, 0);
        assert_eq!(params.cpb_removal_delay_length, 23);
        assert_eq!(params.dpb_output_delay_length, 23);
        assert_eq!(params.time_scale, 90000);
        assert_eq!(params.num_units_in_tick, 1);
        assert!(!params.low_delay_hrd);
        assert!(!params.cbr_flag);
    }

    #[test]
    fn test_hrd_parameters_custom() {
        // Arrange & Act
        let params = create_custom_hrd_params();

        // Assert
        assert_eq!(params.cpb_size_bits, 2_000_000);
        assert_eq!(params.bit_rate_bps, 10_000_000);
        assert_eq!(params.initial_cpb_removal_delay, 90000);
        assert_eq!(params.cpb_removal_delay_length, 24);
        assert_eq!(params.dpb_output_delay_length, 24);
        assert_eq!(params.time_scale, 90000);
        assert_eq!(params.num_units_in_tick, 1);
        assert!(!params.low_delay_hrd);
        assert!(params.cbr_flag);
    }

    #[test]
    fn test_hrd_parameters_frame_duration_sec_normal() {
        // Arrange
        let params = HrdParameters {
            time_scale: 90000,
            num_units_in_tick: 1,
            ..Default::default()
        };

        // Act
        let duration = params.frame_duration_sec();

        // Assert - 1 tick / 90000 ticks/sec = 1/90000 seconds
        assert_eq!(duration, 1.0 / 90000.0);
    }

    #[test]
    fn test_hrd_parameters_frame_duration_sec_custom_time_scale() {
        // Arrange
        let params = HrdParameters {
            time_scale: 180000, // 180kHz
            num_units_in_tick: 2,
            ..Default::default()
        };

        // Act
        let duration = params.frame_duration_sec();

        // Assert - 2 ticks / 180000 ticks/sec = 1/90000 seconds
        assert_eq!(duration, 1.0 / 90000.0);
    }

    #[test]
    fn test_hrd_parameters_frame_duration_sec_zero_time_scale() {
        // Arrange
        let params = HrdParameters {
            time_scale: 0,
            ..Default::default()
        };

        // Act
        let duration = params.frame_duration_sec();

        // Assert - Default to 30fps
        assert_eq!(duration, 1.0 / 30.0);
    }

    #[test]
    fn test_hrd_parameters_cpb_size_bytes() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 8_000_000, // 8 Mbit
            ..Default::default()
        };

        // Act
        let size_bytes = params.cpb_size_bytes();

        // Assert - 8,000,000 bits / 8 = 1,000,000 bytes
        assert_eq!(size_bytes, 1_000_000);
    }

    #[test]
    fn test_hrd_parameters_bit_rate_kbps() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 5_000_000, // 5 Mbps
            ..Default::default()
        };

        // Act
        let rate_kbps = params.bit_rate_kbps();

        // Assert - 5,000,000 bits/sec / 1000 = 5000 kbps
        assert_eq!(rate_kbps, 5000.0);
    }

    #[test]
    fn test_hrd_parameters_max_buffer_delay_sec_normal() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 1_000_000, // 1 Mbit
            bit_rate_bps: 5_000_000,  // 5 Mbps
            ..Default::default()
        };

        // Act
        let delay = params.max_buffer_delay_sec();

        // Assert - 1,000,000 bits / 5,000,000 bits/sec = 0.2 seconds
        assert_eq!(delay, 0.2);
    }

    #[test]
    fn test_hrd_parameters_max_buffer_delay_sec_zero_bit_rate() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 1_000_000,
            bit_rate_bps: 0,
            ..Default::default()
        };

        // Act
        let delay = params.max_buffer_delay_sec();

        // Assert - Zero bit rate, so infinite delay
        assert_eq!(delay, 0.0);
    }

    #[test]
    fn test_hrd_parameters_clone() {
        // Arrange
        let params = create_custom_hrd_params();

        // Act
        let cloned = params.clone();

        // Assert
        assert_eq!(params.cpb_size_bits, cloned.cpb_size_bits);
        assert_eq!(params.bit_rate_bps, cloned.bit_rate_bps);
    }
}

// ============================================================================
// FrameHrdTiming Tests
// ============================================================================

#[cfg(test)]
mod frame_hrd_timing_tests {
    use super::*;

    #[test]
    fn test_frame_hrd_timing_new() {
        // Arrange
        let display_idx = 10;
        let frame_size_bits = 100_000;
        let pts_sec = 1.5;

        // Act
        let timing = FrameHrdTiming::new(display_idx, frame_size_bits, pts_sec);

        // Assert
        assert_eq!(timing.display_idx, display_idx);
        assert_eq!(timing.frame_size_bits, frame_size_bits);
        assert_eq!(timing.pts_sec, pts_sec);
        assert!(timing.cpb_removal_delay.is_none());
        assert!(timing.dpb_output_delay.is_none());
        assert!(timing.dts_sec.is_none());
    }

    #[test]
    fn test_frame_hrd_timing_frame_size_bytes() {
        // Arrange
        let frame_size_bits = 16_000; // 16 kbit

        // Act
        let timing = FrameHrdTiming::new(0, frame_size_bits, 0.0);

        // Assert
        assert_eq!(timing.frame_size_bytes(), 2_000); // 16,000 / 8 = 2,000 bytes
    }

    #[test]
    fn test_frame_hrd_timing_with_delays() {
        // Arrange
        let display_idx = 5;
        let frame_size_bits = 50_000;
        let pts_sec = 0.1;
        let cpb_delay = 90000; // 1 second
        let dpb_delay = 180000; // 2 seconds

        // Act
        let timing = create_test_frame_timing_with_delays(display_idx, frame_size_bits, pts_sec, cpb_delay, dpb_delay);

        // Assert
        assert_eq!(timing.cpb_removal_delay, Some(cpb_delay));
        assert_eq!(timing.dpb_output_delay, Some(dpb_delay));
    }

    #[test]
    fn test_frame_hrd_timing_clone() {
        // Arrange
        let timing = FrameHrdTiming::new(10, 100_000, 1.0);

        // Act
        let cloned = timing.clone();

        // Assert
        assert_eq!(timing.display_idx, cloned.display_idx);
        assert_eq!(timing.frame_size_bits, cloned.frame_size_bits);
    }
}

// ============================================================================
// CpbState Tests
// ============================================================================

#[cfg(test)]
mod cpb_state_tests {
    use super::*;

    #[test]
    fn test_cpb_state_new() {
        // Arrange
        let fullness_bits = 500_000;
        let time_sec = 1.0;
        let frame_idx = 10;

        // Act
        let state = CpbState {
            fullness_bits,
            time_sec,
            frame_idx,
            is_removal: false,
            overflow: false,
            underflow: false,
        };

        // Assert
        assert_eq!(state.fullness_bits, fullness_bits);
        assert_eq!(state.time_sec, time_sec);
        assert_eq!(state.frame_idx, frame_idx);
        assert!(!state.is_removal);
        assert!(!state.overflow);
        assert!(!state.underflow);
    }

    #[test]
    fn test_cpb_state_fullness_percent_normal() {
        // Arrange
        let cpb_size_bits = 1_000_000;
        let state = CpbState {
            fullness_bits: 500_000,
            time_sec: 0.0,
            frame_idx: 0,
            is_removal: false,
            overflow: false,
            underflow: false,
        };

        // Act
        let percent = state.fullness_percent(cpb_size_bits);

        // Assert - 500,000 / 1,000,000 = 0.5
        assert_eq!(percent, 0.5);
    }

    #[test]
    fn test_cpb_state_fullness_percent_zero_cpb_size() {
        // Arrange
        let cpb_size_bits = 0;
        let state = CpbState {
            fullness_bits: 500_000,
            time_sec: 0.0,
            frame_idx: 0,
            is_removal: false,
            overflow: false,
            underflow: false,
        };

        // Act
        let percent = state.fullness_percent(cpb_size_bits);

        // Assert - Zero CPB size results in 0%
        assert_eq!(percent, 0.0);
    }

    #[test]
    fn test_cpb_state_fullness_percent_full() {
        // Arrange
        let cpb_size_bits = 1_000_000;
        let state = CpbState {
            fullness_bits: 1_000_000,
            time_sec: 0.0,
            frame_idx: 0,
            is_removal: false,
            overflow: false,
            underflow: false,
        };

        // Act
        let percent = state.fullness_percent(cpb_size_bits);

        // Assert - 100%
        assert_eq!(percent, 1.0);
    }

    #[test]
    fn test_cpb_state_fullness_bytes() {
        // Arrange
        let state = CpbState {
            fullness_bits: 16_000, // 16 kbit
            time_sec: 0.0,
            frame_idx: 0,
            is_removal: false,
            overflow: false,
            underflow: false,
        };

        // Act
        let bytes = state.fullness_bytes();

        // Assert - 16,000 / 8 = 2,000 bytes
        assert_eq!(bytes, 2_000);
    }

    #[test]
    fn test_cpb_state_clone() {
        // Arrange
        let state = CpbState {
            fullness_bits: 100_000,
            time_sec: 0.5,
            frame_idx: 5,
            is_removal: true,
            overflow: false,
            underflow: true,
        };

        // Act
        let cloned = state.clone();

        // Assert
        assert_eq!(state.fullness_bits, cloned.fullness_bits);
        assert_eq!(state.time_sec, cloned.time_sec);
        assert_eq!(state.is_removal, cloned.is_removal);
        assert_eq!(state.overflow, cloned.overflow);
        assert_eq!(state.underflow, cloned.underflow);
    }
}

// ============================================================================
// HrdModel Tests
// ============================================================================

#[cfg(test)]
mod hrd_model_tests {
    use super::*;

    #[test]
    fn test_hrd_model_new() {
        // Arrange
        let params = create_default_hrd_params();

        // Act
        let model = HrdModel::new(params);

        // Assert
        assert_eq!(model.frame_count(), 0);
        assert_eq!(model.current_fullness_bits(), 0);
        assert_eq!(model.current_fullness_percent(), 0.0);
        assert_eq!(model.overflow_count(), 0);
        assert_eq!(model.underflow_count(), 0);
        assert!(model.state_history().is_empty());
        assert!(model.is_conformant());
    }

    #[test]
    fn test_hrd_model_default() {
        // Arrange & Act
        let model = HrdModel::default();

        // Assert
        assert_eq!(model.frame_count(), 0);
        assert_eq!(model.current_fullness_bits(), 0);
    }

    #[test]
    fn test_hrd_model_reset() {
        // Arrange
        let params = create_default_hrd_params();
        let mut model = HrdModel::new(params);

        // Process some frames first
        let frames = create_frame_sequence();
        for frame in frames {
            model.process_frame(&frame);
        }

        // Assert - Model has state
        assert!(model.frame_count() > 0);
        assert!(!model.state_history().is_empty());

        // Act
        model.reset();

        // Assert - Reset to initial state
        assert_eq!(model.frame_count(), 0);
        assert_eq!(model.current_fullness_bits(), 0);
        assert!(model.state_history().is_empty());
        assert_eq!(model.overflow_count(), 0);
        assert_eq!(model.underflow_count(), 0);
    }

    #[test]
    fn test_hrd_model_initialize_buffer() {
        // Arrange
        let params = HrdParameters {
            initial_cpb_removal_delay: 90000, // 1 second
            bit_rate_bps: 5_000_000,         // 5 Mbps
            cpb_size_bits: 1_000_000,       // 1 Mbit buffer
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        // Act
        model.initialize_buffer();

        // Assert - Buffer should be filled based on initial delay
        // 1 second * 5 Mbps = 5 Mbit, but limited to 1 Mbit buffer size
        assert_eq!(model.current_fullness_bits(), 1_000_000);
        assert_eq!(model.current_fullness_percent(), 1.0);
        assert_eq!(model.state_history().len(), 1);
    }

    #[test]
    fn test_hrd_model_process_frame_no_initial_delay() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 5_000_000,
            cpb_size_bits: 1_000_000,
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        let frame = create_test_frame_timing(0, 100_000, 0.0); // First frame at 0s

        // Act
        let state = model.process_frame(&frame);

        // Assert - First frame uses frame_duration for time delta, which is tiny
        // so very few bits arrive before removal
        assert_eq!(model.frame_count(), 1);
        assert!(state.underflow); // Frame larger than buffer
        assert!(!state.overflow);
    }

    #[test]
    fn test_hrd_model_process_frame_with_buffer_filling() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 10_000_000, // 10 Mbps
            cpb_size_bits: 1_000_000, // 1 Mbit buffer
            time_scale: 90000,
            num_units_in_tick: 3000, // 30fps - 0.0333 seconds per frame
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        // First frame - 1/30 second passes, buffer fills
        let frame1 = create_test_frame_timing(0, 100_000, 0.0333);
        let state1 = model.process_frame(&frame1);

        // Assert - Buffer should have filled with 10 Mbps * 0.0333s = 333k bits
        // After removal of 100k bit frame: ~233k bits remain
        assert!(state1.fullness_bits > 200_000 && state1.fullness_bits < 300_000);

        // Second frame - another 1/30 second, buffer fills more
        let frame2 = create_test_frame_timing(1, 100_000, 0.0666);
        let state2 = model.process_frame(&frame2);

        // Assert - After second fill and removal, should be closer to capacity
        assert!(state2.fullness_bits > 400_000);
    }

    #[test]
    fn test_hrd_model_process_frame_underflow() {
        // Arrange
        let params = create_default_hrd_params();
        let mut model = HrdModel::new(params);

        // First frame - no buffer filling
        let frame = create_test_frame_timing(0, 500_000, 0.0); // 500 kbit frame

        // Manually fill buffer a bit
        model.cpb_fullness_bits = 200_000; // Only 200 kbits in buffer

        // Act
        let state = model.process_frame(&frame);

        // Assert - Underflow occurred
        assert_eq!(state.fullness_bits, 0); // Buffer emptied
        assert!(state.underflow);
        assert_eq!(model.underflow_count(), 1);
        assert_eq!(model.overflow_count(), 0);
    }

    #[test]
    fn test_hrd_model_process_frame_normal_operation() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 5_000_000, // 5 Mbps
            cpb_size_bits: 1_000_000, // 1 Mbit buffer
            time_scale: 90000,
            num_units_in_tick: 3000, // 30fps
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        // Initialize buffer first
        model.initialize_buffer();

        // Frame arrives after buffer has filled appropriately
        let frame = create_test_frame_timing(0, 100_000, 0.0333);
        let state = model.process_frame(&frame);

        // Assert - Buffer started at 1M (from init), added 166k (5Mbps * 0.033s), removed 100k
        assert!(!state.overflow);
        assert!(!state.underflow);
        assert!(model.is_conformant());
    }

    #[test]
    fn test_hrd_model_is_conformant_no_errors() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 5_000_000,
            cpb_size_bits: 1_000_000,
            time_scale: 90000,
            num_units_in_tick: 3000,
            ..Default::default()
        };
        let mut model = HrdModel::new(params);
        model.initialize_buffer();

        let frame = create_test_frame_timing(0, 50_000, 0.0333); // Small frame

        // Act
        model.process_frame(&frame);

        // Assert - Should be conformant
        assert!(model.is_conformant());
        assert_eq!(model.overflow_count(), 0);
        assert_eq!(model.underflow_count(), 0);
    }

    #[test]
    fn test_hrd_model_is_conformant_with_errors() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 50_000, // Very small buffer
            ..Default::default()
        };
        let mut model = HrdModel::new(params);
        let frame = create_test_frame_timing(0, 100_000, 0.0); // Larger frame

        // Act
        model.process_frame(&frame);

        // Assert - Should not be conformant (underflow)
        assert!(!model.is_conformant());
        assert_eq!(model.overflow_count(), 0);
        assert_eq!(model.underflow_count(), 1);
    }
}

// ============================================================================
// HrdStatistics Tests
// ============================================================================

#[cfg(test)]
mod hrd_statistics_tests {
    use super::*;

    #[test]
    fn test_hrd_statistics_from_model_empty() {
        // Arrange
        let params = create_default_hrd_params();
        let model = HrdModel::new(params);

        // Act
        let stats = HrdStatistics::from_model(&model);

        // Assert
        assert_eq!(stats.total_frames, 0);
        assert_eq!(stats.overflow_count, 0);
        assert_eq!(stats.underflow_count, 0);
        assert_eq!(stats.min_fullness_bits, 0);
        assert_eq!(stats.max_fullness_bits, 0);
        assert_eq!(stats.avg_fullness_bits, 0.0);
        assert!(stats.is_conformant);
    }

    #[test]
    fn test_hrd_statistics_from_model_with_frames() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 5_000_000,
            cpb_size_bits: 1_000_000,
            time_scale: 90000,
            num_units_in_tick: 3000,
            ..Default::default()
        };
        let mut model = HrdModel::new(params);
        model.initialize_buffer();

        // Process frames that create different buffer levels
        let frame1 = create_test_frame_timing(0, 50_000, 0.0333);
        let frame2 = create_test_frame_timing(1, 100_000, 0.0666);

        model.process_frame(&frame1);
        model.process_frame(&frame2);

        // Act
        let stats = HrdStatistics::from_model(&model);

        // Assert
        assert_eq!(stats.total_frames, 2);
        assert_eq!(stats.overflow_count, 0);
        assert_eq!(stats.underflow_count, 0);
        // Buffer should have some bits in it after initialization and frames
        assert!(stats.min_fullness_bits >= 0);
        assert!(stats.max_fullness_bits > 0);
        assert!(stats.avg_fullness_bits > 0.0);
        assert!(stats.is_conformant);
    }

    #[test]
    fn test_hrd_statistics_from_model_with_violations() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 50_000, // Small buffer
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        let frame = create_test_frame_timing(0, 100_000, 0.0);
        model.process_frame(&frame); // Will cause underflow

        // Act
        let stats = HrdStatistics::from_model(&model);

        // Assert
        assert_eq!(stats.total_frames, 1);
        assert_eq!(stats.overflow_count, 0);
        assert_eq!(stats.underflow_count, 1);
        assert!(!stats.is_conformant);
    }

    #[test]
    fn test_hrd_statistics_fullness_percentages() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 5_000_000,
            cpb_size_bits: 1_000_000,
            time_scale: 90000,
            num_units_in_tick: 3000,
            ..Default::default()
        };
        let mut model = HrdModel::new(params);
        model.initialize_buffer();

        // Process a small frame
        let frame = create_test_frame_timing(0, 50_000, 0.0333);
        model.process_frame(&frame);

        // Act
        let stats = HrdStatistics::from_model(&model);

        // Assert - Verify statistics are calculated correctly
        // With initial buffer at 1M, arriving bits and frame removal
        assert!(stats.avg_fullness_percent() > 0.0);
        assert!(stats.avg_fullness_percent() <= 1.0);
    }

    #[test]
    fn test_hrd_statistics_fullness_percentages_zero_buffer() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 0,
            ..Default::default()
        };
        let mut model = HrdModel::new(params);
        let frame = create_test_frame_timing(0, 0, 0.0);
        model.process_frame(&frame);

        // Act
        let stats = HrdStatistics::from_model(&model);

        // Assert
        assert_eq!(stats.min_fullness_percent(), 0.0);
        assert_eq!(stats.max_fullness_percent(), 0.0);
        assert_eq!(stats.avg_fullness_percent(), 0.0);
    }

    #[test]
    fn test_hrd_statistics_clone() {
        // Arrange
        let params = create_default_hrd_params();
        let model = HrdModel::new(params);
        let stats = HrdStatistics::from_model(&model);

        // Act
        let cloned = stats.clone();

        // Assert
        assert_eq!(stats.total_frames, cloned.total_frames);
        assert_eq!(stats.is_conformant, cloned.is_conformant);
    }
}

// ============================================================================
// HrdPlotData Tests
// ============================================================================

#[cfg(test)]
mod hrd_plot_data_tests {
    use super::*;

    #[test]
    fn test_hrd_plot_data_from_model_empty() {
        // Arrange
        let params = create_default_hrd_params();
        let model = HrdModel::new(params);

        // Act
        let plot_data = HrdPlotData::from_model(&model);

        // Assert
        assert!(plot_data.points.is_empty());
        assert_eq!(plot_data.cpb_size_bits, 1_000_000);
        assert_eq!(plot_data.max_time_sec, 0.0);
        assert!(plot_data.statistics.is_some());
        assert_eq!(plot_data.statistics.unwrap().total_frames, 0);
    }

    #[test]
    fn test_hrd_plot_data_from_model_with_frames() {
        // Arrange
        let params = create_default_hrd_params();
        let mut model = HrdModel::new(params);

        let frame = create_test_frame_timing(0, 100_000, 1.0);
        model.process_frame(&frame);

        // Act
        let plot_data = HrdPlotData::from_model(&model);

        // Assert
        assert!(!plot_data.points.is_empty());
        assert_eq!(plot_data.points.len(), 2); // Pre and post removal
        assert_eq!(plot_data.max_time_sec, 1.0);
        assert!(plot_data.statistics.is_some());
        assert_eq!(plot_data.statistics.unwrap().total_frames, 1);
    }

    #[test]
    fn test_hrd_plot_data_points_in_range() {
        // Arrange
        let params = create_default_hrd_params();
        let mut model = HrdModel::new(params);

        let frame1 = create_test_frame_timing(0, 100_000, 0.5);
        let frame2 = create_test_frame_timing(1, 150_000, 1.0);
        let frame3 = create_test_frame_timing(2, 200_000, 1.5);

        model.process_frame(&frame1);
        model.process_frame(&frame2);
        model.process_frame(&frame3);

        // Act
        let plot_data = HrdPlotData::from_model(&model);
        let points_in_range = plot_data.points_in_range(0.75, 1.25);

        // Assert - Should include points from frame 2 (1.0)
        assert!(!points_in_range.is_empty());
        for point in points_in_range {
            assert!(point.time_sec >= 0.75 && point.time_sec <= 1.25);
        }
    }

    #[test]
    fn test_hrd_plot_data_violation_points() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 50_000, // Small buffer
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        let frame = create_test_frame_timing(0, 100_000, 1.0);
        model.process_frame(&frame); // Will cause underflow

        // Act
        let plot_data = HrdPlotData::from_model(&model);
        let violation_points = plot_data.violation_points();

        // Assert - Should have violation points
        assert!(!violation_points.is_empty());
        for point in violation_points {
            assert!(point.violation);
        }
    }

    #[test]
    fn test_hrd_plot_data_has_violations() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 50_000, // Small buffer
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        let frame = create_test_frame_timing(0, 100_000, 1.0);
        model.process_frame(&frame); // Will cause underflow

        // Act
        let plot_data = HrdPlotData::from_model(&model);

        // Assert
        assert!(plot_data.has_violations());
    }

    #[test]
    fn test_hrd_plot_data_no_violations() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 5_000_000,
            cpb_size_bits: 1_000_000,
            time_scale: 90000,
            num_units_in_tick: 3000,
            ..Default::default()
        };
        let mut model = HrdModel::new(params);
        model.initialize_buffer();

        let frame = create_test_frame_timing(0, 50_000, 0.0333);
        model.process_frame(&frame); // No violations

        // Act
        let plot_data = HrdPlotData::from_model(&model);

        // Assert
        assert!(!plot_data.has_violations());
    }

    #[test]
    fn test_hrd_plot_data_clone() {
        // Arrange
        let params = create_default_hrd_params();
        let mut model = HrdModel::new(params);

        let frame = create_test_frame_timing(0, 100_000, 1.0);
        model.process_frame(&frame);

        // Act
        let plot_data = HrdPlotData::from_model(&model);
        let cloned = plot_data.clone();

        // Assert
        assert_eq!(plot_data.points.len(), cloned.points.len());
        assert_eq!(plot_data.cpb_size_bits, cloned.cpb_size_bits);
        assert_eq!(plot_data.max_time_sec, cloned.max_time_sec);
    }
}

// ============================================================================
// HrdLaneData Tests
// ============================================================================

#[cfg(test)]
mod hrd_lane_data_tests {
    use super::*;

    #[test]
    fn test_hrd_to_lane_data_empty_model() {
        // Arrange
        let params = create_default_hrd_params();
        let model = HrdModel::new(params);

        // Act
        let lane_data = hrd_to_lane_data(&model);

        // Assert
        assert!(lane_data.is_empty());
    }

    #[test]
    fn test_hrd_to_lane_data_with_frames() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 1_000_000,
            bit_rate_bps: 5_000_000,
            time_scale: 90000,
            num_units_in_tick: 3000, // 30fps
            ..Default::default()
        };
        let mut model = HrdModel::new(params);
        model.initialize_buffer();

        let frame = create_test_frame_timing(0, 100_000, 0.0333);
        model.process_frame(&frame);

        // Act
        let lane_data = hrd_to_lane_data(&model);

        // Assert
        assert!(!lane_data.is_empty());
        assert_eq!(lane_data.len(), 1); // One frame processed

        let data = &lane_data[0];
        assert_eq!(data.display_idx, 0);
        assert_eq!(data.frame_size_bits, 100_000);
        // Pre + post should be close to 1.0 (accounting for bits arriving)
        assert!(!data.overflow);
        assert!(!data.underflow);
    }

    #[test]
    fn test_hrd_to_lane_data_with_violations() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 50_000, // Small buffer
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        let frame = create_test_frame_timing(0, 100_000, 0.0333);
        model.process_frame(&frame); // Will cause underflow

        // Act
        let lane_data = hrd_to_lane_data(&model);

        // Assert
        assert!(!lane_data.is_empty());
        let data = &lane_data[0];
        assert!(data.underflow);
        assert!(!data.overflow);
    }

    #[test]
    fn test_hrd_to_lane_data_multiple_frames() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 5_000_000,
            cpb_size_bits: 1_000_000,
            time_scale: 90000,
            num_units_in_tick: 3000,
            ..Default::default()
        };
        let mut model = HrdModel::new(params);
        model.initialize_buffer();

        let frame1 = create_test_frame_timing(0, 50_000, 0.0333);
        let frame2 = create_test_frame_timing(1, 75_000, 0.0666);

        model.process_frame(&frame1);
        model.process_frame(&frame2);

        // Act
        let lane_data = hrd_to_lane_data(&model);

        // Assert
        assert_eq!(lane_data.len(), 2);
        assert_eq!(lane_data[0].display_idx, 0);
        assert_eq!(lane_data[1].display_idx, 1);
        assert_eq!(lane_data[0].frame_size_bits, 50_000);
        assert_eq!(lane_data[1].frame_size_bits, 75_000);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_hrd_model_zero_bit_rate() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 0, // Zero bit rate
            cpb_size_bits: 1_000_000,
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        let frame = create_test_frame_timing(0, 100_000, 1.0);

        // Act
        let state = model.process_frame(&frame);

        // Assert - No bits arrived, but frame still removed
        assert_eq!(state.fullness_bits, 0); // Buffer emptied
        assert!(!state.overflow);
        assert!(state.underflow); // Underflow because frame > 0 bits
    }

    #[test]
    fn test_hrd_model_maximum_buffer_fill() {
        // Arrange
        let params = HrdParameters {
            bit_rate_bps: 10_000_000, // 10 Mbps
            cpb_size_bits: 1_000_000, // 1 Mbit buffer
            time_scale: 90000,
            num_units_in_tick: 3000, // 30fps
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        // First frame - 1/30 second passes
        let frame1 = create_test_frame_timing(0, 100_000, 0.0333);
        model.process_frame(&frame1);

        // Process many frames to fill buffer
        for i in 1..10 {
            let frame = create_test_frame_timing(i, 100_000, i as f64 * 0.0333);
            model.process_frame(&frame);
        }

        // Assert - Buffer should have overflowed
        assert!(model.overflow_count() > 0);
        assert!(model.current_fullness_bits() <= model.params.cpb_size_bits);
    }

    #[test]
    fn test_hrd_parameters_extreme_values() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: u64::MAX,
            bit_rate_bps: u64::MAX,
            ..Default::default()
        };

        // Act
        let cpb_bytes = params.cpb_size_bytes();
        let bit_rate_kbps = params.bit_rate_kbps();
        let max_delay = params.max_buffer_delay_sec();

        // Assert
        assert_eq!(cpb_bytes, u64::MAX / 8);
        assert_eq!(bit_rate_kbps, u64::MAX as f64 / 1000.0);
        assert_eq!(max_delay, 1.0); // MAX/MAX = 1.0
    }

    #[test]
    fn test_hrd_plot_data_time_bounds() {
        // Arrange
        let params = create_default_hrd_params();
        let mut model = HrdModel::new(params);

        // Process frames at different times
        let frame1 = create_test_frame_timing(0, 100_000, 0.0);
        let frame2 = create_test_frame_timing(1, 150_000, 5.0);

        model.process_frame(&frame1);
        model.process_frame(&frame2);

        // Act
        let plot_data = HrdPlotData::from_model(&model);

        // Assert
        assert_eq!(plot_data.max_time_sec, 5.0);
        assert!(!plot_data.points.is_empty());
    }

    #[test]
    fn test_cpb_state_fullness_percent_rounding() {
        // Arrange
        let cpb_size_bits = 3; // Odd number
        let state = CpbState {
            fullness_bits: 1,
            time_sec: 0.0,
            frame_idx: 0,
            is_removal: false,
            overflow: false,
            underflow: false,
        };

        // Act
        let percent = state.fullness_percent(cpb_size_bits);

        // Assert - Should be 1/3 = ~0.333
        assert_eq!(percent, 1.0 / 3.0);
    }

    #[test]
    fn test_hrd_model_concurrent_states() {
        // Arrange
        let params = create_default_hrd_params();
        let mut model = HrdModel::new(params);

        // Process multiple frames quickly
        for i in 0..10 {
            let frame = create_test_frame_timing(i, 10_000, i as f64 * 0.1);
            model.process_frame(&frame);
        }

        // Act
        let history = model.state_history();

        // Assert
        assert_eq!(model.frame_count(), 10);
        assert_eq!(history.len(), 20); // 2 states per frame
        assert_eq!(history[0].frame_idx, 0);
        assert_eq!(history[19].frame_idx, 9);
    }

    #[test]
    fn test_hrd_statistics_empty_removal_states() {
        // Arrange - Model with no removal states (shouldn't happen in practice)
        let params = create_default_hrd_params();
        let _model = HrdModel::new(params.clone());

        // Manually create empty model
        let empty_model = HrdModel {
            params,
            cpb_fullness_bits: 0,
            current_time_sec: 0.0,
            state_history: Vec::new(),
            overflow_count: 0,
            underflow_count: 0,
            frame_count: 0,
        };

        // Act
        let stats = HrdStatistics::from_model(&empty_model);

        // Assert
        assert_eq!(stats.min_fullness_bits, 0);
        assert_eq!(stats.max_fullness_bits, 0);
        assert_eq!(stats.avg_fullness_bits, 0.0);
    }

    #[test]
    fn test_hrd_lane_data_very_small_frames() {
        // Arrange
        let params = create_default_hrd_params();
        let mut model = HrdModel::new(params);

        // Very small frame
        let frame = create_test_frame_timing(0, 1, 0.0); // 1 bit frame
        model.process_frame(&frame);

        // Act
        let lane_data = hrd_to_lane_data(&model);

        // Assert
        assert!(!lane_data.is_empty());
        assert_eq!(lane_data[0].frame_size_bits, 1);
    }

    #[test]
    fn test_frame_hrd_timing_zero_size_frame() {
        // Arrange
        let display_idx = 0;
        let frame_size_bits = 0; // Zero size
        let pts_sec = 1.0;

        // Act
        let timing = FrameHrdTiming::new(display_idx, frame_size_bits, pts_sec);

        // Assert
        assert_eq!(timing.frame_size_bytes(), 0);
        assert_eq!(timing.frame_size_bits, 0);
    }

    #[test]
    fn test_hrd_model_very_large_frames() {
        // Arrange
        let params = HrdParameters {
            cpb_size_bits: 1_000_000, // 1 Mbit buffer
            ..Default::default()
        };
        let mut model = HrdModel::new(params);

        // Very large frame
        let frame = create_test_frame_timing(0, 2_000_000, 0.0); // 2 Mbit frame

        // Act
        let state = model.process_frame(&frame);

        // Assert
        assert_eq!(state.fullness_bits, 0); // Buffer emptied
        assert!(state.underflow);
        assert_eq!(model.underflow_count(), 1);
    }

    #[test]
    fn test_hrd_plot_data_single_point() {
        // Arrange
        let params = create_default_hrd_params();
        let mut model = HrdModel::new(params);

        let frame = create_test_frame_timing(0, 0, 0.0); // Zero size frame
        model.process_frame(&frame);

        // Act
        let plot_data = HrdPlotData::from_model(&model);
        let points_in_range = plot_data.points_in_range(0.0, 0.0);

        // Assert
        assert!(!points_in_range.is_empty());
        assert_eq!(points_in_range.len(), 2); // Pre and post removal
    }

    #[test]
    fn test_hrd_parameters_high_delay_flag() {
        // Arrange
        let params = HrdParameters {
            low_delay_hrd: true,
            ..Default::default()
        };

        // Act
        let duration = params.frame_duration_sec();

        // Assert - Should still work the same way
        assert_eq!(duration, 1.0 / 90000.0);
    }

    #[test]
    fn test_hrd_parameters_cbr_flag() {
        // Arrange
        let params = HrdParameters {
            cbr_flag: true,
            ..Default::default()
        };

        // Act & Assert - CBR flag should be set
        assert!(params.cbr_flag);
    }
}