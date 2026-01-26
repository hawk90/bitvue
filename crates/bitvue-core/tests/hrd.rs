//! Tests for hrd module

use bitvue_core::{
    hrd_to_lane_data, CpbState, FrameHrdTiming, HrdModel, HrdParameters, HrdPlotData, HrdStatistics,
};

#[test]
fn test_hrd_parameters_default() {
    let params = HrdParameters::default();
    assert_eq!(params.cpb_size_bits, 1_000_000);
    assert_eq!(params.bit_rate_bps, 5_000_000);
    assert!((params.frame_duration_sec() - 1.0 / 90000.0).abs() < 0.0001);
}

#[test]
fn test_hrd_parameters_calculations() {
    let params = HrdParameters {
        cpb_size_bits: 8_000_000, // 8 Mbit = 1 MB
        bit_rate_bps: 10_000_000, // 10 Mbps
        time_scale: 30000,
        num_units_in_tick: 1001,
        ..Default::default()
    };

    assert_eq!(params.cpb_size_bytes(), 1_000_000);
    assert!((params.bit_rate_kbps() - 10_000.0).abs() < 0.1);
    assert!((params.max_buffer_delay_sec() - 0.8).abs() < 0.01);
}

#[test]
fn test_frame_hrd_timing() {
    let timing = FrameHrdTiming::new(0, 80_000, 0.0);
    assert_eq!(timing.display_idx, 0);
    assert_eq!(timing.frame_size_bits, 80_000);
    assert_eq!(timing.frame_size_bytes(), 10_000);
}

#[test]
fn test_cpb_state_fullness() {
    let state = CpbState {
        fullness_bits: 500_000,
        time_sec: 1.0,
        frame_idx: 10,
        is_removal: true,
        overflow: false,
        underflow: false,
    };

    assert!((state.fullness_percent(1_000_000) - 0.5).abs() < 0.001);
    assert_eq!(state.fullness_bytes(), 62_500);
}

#[test]
fn test_hrd_model_basic() {
    let params = HrdParameters {
        cpb_size_bits: 1_000_000,
        bit_rate_bps: 1_000_000, // 1 Mbps
        time_scale: 30,
        num_units_in_tick: 1, // 30 fps
        ..Default::default()
    };
    let mut model = HrdModel::new(params);

    // Process first frame (100kbit at t=0)
    let timing = FrameHrdTiming::new(0, 100_000, 0.0);
    model.process_frame(&timing);

    assert_eq!(model.frame_count(), 1);
}

#[test]
fn test_hrd_model_no_overflow() {
    let params = HrdParameters {
        cpb_size_bits: 2_000_000, // 2 Mbit
        bit_rate_bps: 3_000_000,  // 3 Mbps - faster fill than drain
        time_scale: 30,
        num_units_in_tick: 1,
        ..Default::default()
    };
    let mut model = HrdModel::new(params);

    // Process 10 frames at 30kbit each, 1/30 sec apart
    // Arrival rate: 3Mbps / 30fps = 100kbit per frame interval
    // Frame size: 30kbit - well under arrival rate
    for i in 0..10 {
        let timing = FrameHrdTiming::new(i, 30_000, i as f64 / 30.0);
        model.process_frame(&timing);
    }

    assert_eq!(model.frame_count(), 10);
    assert!(model.is_conformant());
    assert_eq!(model.overflow_count(), 0);
    assert_eq!(model.underflow_count(), 0);
}

#[test]
fn test_hrd_model_overflow() {
    let params = HrdParameters {
        cpb_size_bits: 100_000,   // Very small buffer (100kbit)
        bit_rate_bps: 10_000_000, // 10 Mbps
        time_scale: 30,
        num_units_in_tick: 1,
        ..Default::default()
    };
    let mut model = HrdModel::new(params);

    // Process frames - buffer should overflow
    for i in 0..10 {
        let timing = FrameHrdTiming::new(i, 10_000, i as f64 / 30.0);
        model.process_frame(&timing);
    }

    // Should have overflows because buffer fills faster than frames are removed
    assert!(model.overflow_count() > 0);
}

#[test]
fn test_hrd_model_underflow() {
    let params = HrdParameters {
        cpb_size_bits: 1_000_000,
        bit_rate_bps: 100_000, // Very slow fill rate (100 kbps)
        time_scale: 30,
        num_units_in_tick: 1,
        ..Default::default()
    };
    let mut model = HrdModel::new(params);

    // Large frames with slow fill - should underflow
    for i in 0..5 {
        let timing = FrameHrdTiming::new(i, 500_000, i as f64 / 30.0); // 500kbit frames
        model.process_frame(&timing);
    }

    // Should have underflows
    assert!(model.underflow_count() > 0);
}

#[test]
fn test_hrd_statistics() {
    let params = HrdParameters {
        cpb_size_bits: 1_000_000,
        bit_rate_bps: 1_000_000,
        time_scale: 30,
        num_units_in_tick: 1,
        ..Default::default()
    };
    let mut model = HrdModel::new(params);

    for i in 0..5 {
        let timing = FrameHrdTiming::new(i, 30_000, i as f64 / 30.0);
        model.process_frame(&timing);
    }

    let stats = HrdStatistics::from_model(&model);
    assert_eq!(stats.total_frames, 5);
    assert_eq!(stats.cpb_size_bits, 1_000_000);
}

#[test]
fn test_hrd_plot_data() {
    let params = HrdParameters {
        cpb_size_bits: 1_000_000,
        bit_rate_bps: 1_000_000,
        time_scale: 30,
        num_units_in_tick: 1,
        ..Default::default()
    };
    let mut model = HrdModel::new(params);

    for i in 0..3 {
        let timing = FrameHrdTiming::new(i, 30_000, i as f64 / 30.0);
        model.process_frame(&timing);
    }

    let plot = HrdPlotData::from_model(&model);
    assert!(!plot.points.is_empty());
    assert!(plot.statistics.is_some());
}

#[test]
fn test_hrd_plot_violation_points() {
    let params = HrdParameters {
        cpb_size_bits: 100_000,   // Small buffer
        bit_rate_bps: 10_000_000, // High rate
        time_scale: 30,
        num_units_in_tick: 1,
        ..Default::default()
    };
    let mut model = HrdModel::new(params);

    for i in 0..5 {
        let timing = FrameHrdTiming::new(i, 10_000, i as f64 / 30.0);
        model.process_frame(&timing);
    }

    let plot = HrdPlotData::from_model(&model);
    assert!(plot.has_violations());
    assert!(!plot.violation_points().is_empty());
}

#[test]
fn test_hrd_lane_data() {
    let params = HrdParameters {
        cpb_size_bits: 1_000_000,
        bit_rate_bps: 1_000_000,
        time_scale: 30,
        num_units_in_tick: 1,
        ..Default::default()
    };
    let mut model = HrdModel::new(params);

    for i in 0..3 {
        let timing = FrameHrdTiming::new(i, 30_000, i as f64 / 30.0);
        model.process_frame(&timing);
    }

    let lane_data = hrd_to_lane_data(&model);
    assert_eq!(lane_data.len(), 3);
    assert_eq!(lane_data[0].display_idx, 0);
    assert!(lane_data[0].pre_removal_percent >= lane_data[0].post_removal_percent);
}

#[test]
fn test_hrd_model_reset() {
    let mut model = HrdModel::default();

    let timing = FrameHrdTiming::new(0, 50_000, 0.0);
    model.process_frame(&timing);
    assert_eq!(model.frame_count(), 1);

    model.reset();
    assert_eq!(model.frame_count(), 0);
    assert_eq!(model.current_fullness_bits(), 0);
    assert!(model.state_history().is_empty());
}

#[test]
fn test_hrd_conformance_check() {
    let params = HrdParameters {
        cpb_size_bits: 2_000_000,
        bit_rate_bps: 2_000_000,
        time_scale: 30,
        num_units_in_tick: 1,
        ..Default::default()
    };
    let mut model = HrdModel::new(params);

    // Process well-behaved stream
    for i in 0..10 {
        let timing = FrameHrdTiming::new(i, 50_000, i as f64 / 30.0);
        model.process_frame(&timing);
    }

    assert!(model.is_conformant());
}
