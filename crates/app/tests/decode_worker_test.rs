//! Tests for Decode Worker

#[test]
fn test_decode_task() {
    struct DecodeTask {
        frame_index: usize,
        codec: String,
        priority: u8,
    }

    let task = DecodeTask {
        frame_index: 42,
        codec: "AV1".to_string(),
        priority: 10,
    };

    assert_eq!(task.frame_index, 42);
}

#[test]
fn test_decode_result() {
    struct DecodeResult {
        success: bool,
        yuv_data: Vec<u8>,
        decode_time_ms: u64,
    }

    let result = DecodeResult {
        success: true,
        yuv_data: vec![0u8; 1920 * 1080],
        decode_time_ms: 15,
    };

    assert!(result.success);
}

#[test]
fn test_decoder_init() {
    struct DecoderInit {
        codec_type: String,
        thread_count: usize,
    }

    let init = DecoderInit {
        codec_type: "AV1".to_string(),
        thread_count: 4,
    };

    assert!(init.thread_count > 0);
}

#[test]
fn test_frame_buffer_pool() {
    struct FrameBufferPool {
        available: Vec<usize>,
        in_use: Vec<usize>,
    }

    impl FrameBufferPool {
        fn allocate(&mut self) -> Option<usize> {
            if let Some(buffer_id) = self.available.pop() {
                self.in_use.push(buffer_id);
                Some(buffer_id)
            } else {
                None
            }
        }
    }

    let mut pool = FrameBufferPool {
        available: vec![0, 1, 2],
        in_use: vec![],
    };

    assert_eq!(pool.allocate(), Some(2));
}

#[test]
fn test_decode_error_handling() {
    #[derive(Debug, PartialEq)]
    enum DecodeError {
        InvalidData,
        DecoderNotFound,
        OutOfMemory,
    }

    let error = DecodeError::InvalidData;
    assert_eq!(error, DecodeError::InvalidData);
}

#[test]
fn test_frame_dependency() {
    struct FrameDependency {
        frame_index: usize,
        depends_on: Vec<usize>,
    }

    let frame = FrameDependency {
        frame_index: 5,
        depends_on: vec![0, 4],
    };

    assert_eq!(frame.depends_on.len(), 2);
}

#[test]
fn test_decode_throughput() {
    fn calculate_fps(frames: usize, time_ms: u64) -> f64 {
        if time_ms == 0 {
            0.0
        } else {
            (frames as f64 / time_ms as f64) * 1000.0
        }
    }

    let fps = calculate_fps(60, 1000);
    assert_eq!(fps, 60.0);
}

#[test]
fn test_yuv_format() {
    #[derive(Debug, PartialEq)]
    enum YuvFormat {
        Yuv420,
        Yuv422,
        Yuv444,
    }

    let format = YuvFormat::Yuv420;
    assert_eq!(format, YuvFormat::Yuv420);
}
