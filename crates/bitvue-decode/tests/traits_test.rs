#![allow(dead_code)]
//! Tests for Decoder Traits

#[test]
fn test_decoder_trait() {
    trait VideoDecoder {
        fn initialize(&mut self) -> Result<(), String>;
        fn decode_frame(&mut self, data: &[u8]) -> Result<Vec<u8>, String>;
        fn flush(&mut self) -> Result<Vec<Vec<u8>>, String>;
        fn reset(&mut self);
    }

    struct MockDecoder {
        initialized: bool,
    }

    impl VideoDecoder for MockDecoder {
        fn initialize(&mut self) -> Result<(), String> {
            self.initialized = true;
            Ok(())
        }

        fn decode_frame(&mut self, data: &[u8]) -> Result<Vec<u8>, String> {
            if !self.initialized {
                return Err("Not initialized".to_string());
            }
            Ok(data.to_vec())
        }

        fn flush(&mut self) -> Result<Vec<Vec<u8>>, String> {
            Ok(vec![])
        }

        fn reset(&mut self) {
            self.initialized = false;
        }
    }

    let mut decoder = MockDecoder { initialized: false };
    assert!(decoder.initialize().is_ok());
    assert!(decoder.decode_frame(&[1, 2, 3]).is_ok());
}

#[test]
fn test_frame_sink_trait() {
    trait FrameSink {
        fn accept_frame(&mut self, frame: Vec<u8>) -> bool;
        fn frame_count(&self) -> usize;
    }

    struct FrameCollector {
        frames: Vec<Vec<u8>>,
    }

    impl FrameSink for FrameCollector {
        fn accept_frame(&mut self, frame: Vec<u8>) -> bool {
            self.frames.push(frame);
            true
        }

        fn frame_count(&self) -> usize {
            self.frames.len()
        }
    }

    let mut collector = FrameCollector { frames: vec![] };
    assert!(collector.accept_frame(vec![1, 2, 3]));
    assert_eq!(collector.frame_count(), 1);
}

#[test]
fn test_codec_info_trait() {
    trait CodecInfo {
        fn codec_name(&self) -> &str;
        fn supports_resolution(&self, width: usize, height: usize) -> bool;
        fn max_bit_depth(&self) -> u8;
    }

    struct AV1Info;

    impl CodecInfo for AV1Info {
        fn codec_name(&self) -> &str {
            "AV1"
        }

        fn supports_resolution(&self, width: usize, height: usize) -> bool {
            width <= 7680 && height <= 4320
        }

        fn max_bit_depth(&self) -> u8 {
            12
        }
    }

    let info = AV1Info;
    assert_eq!(info.codec_name(), "AV1");
    assert!(info.supports_resolution(1920, 1080));
    assert_eq!(info.max_bit_depth(), 12);
}

#[test]
fn test_frame_allocator_trait() {
    trait FrameAllocator {
        fn allocate(&mut self, width: usize, height: usize) -> Option<usize>;
        fn deallocate(&mut self, id: usize);
    }

    struct SimpleAllocator {
        next_id: usize,
        allocated: Vec<usize>,
    }

    impl FrameAllocator for SimpleAllocator {
        fn allocate(&mut self, _width: usize, _height: usize) -> Option<usize> {
            let id = self.next_id;
            self.next_id += 1;
            self.allocated.push(id);
            Some(id)
        }

        fn deallocate(&mut self, id: usize) {
            if let Some(pos) = self.allocated.iter().position(|&x| x == id) {
                self.allocated.remove(pos);
            }
        }
    }

    let mut allocator = SimpleAllocator {
        next_id: 0,
        allocated: vec![],
    };

    let id = allocator.allocate(1920, 1080).unwrap();
    assert_eq!(id, 0);
    allocator.deallocate(id);
    assert!(allocator.allocated.is_empty());
}

#[test]
fn test_error_handler_trait() {
    trait ErrorHandler {
        fn on_error(&mut self, error: String);
        fn error_count(&self) -> usize;
    }

    struct ErrorCollector {
        errors: Vec<String>,
    }

    impl ErrorHandler for ErrorCollector {
        fn on_error(&mut self, error: String) {
            self.errors.push(error);
        }

        fn error_count(&self) -> usize {
            self.errors.len()
        }
    }

    let mut handler = ErrorCollector { errors: vec![] };
    handler.on_error("Decode failed".to_string());
    assert_eq!(handler.error_count(), 1);
}

#[test]
fn test_decoder_factory_trait() {
    trait DecoderFactory {
        fn create_decoder(&self, codec: &str) -> Option<String>;
        fn supported_codecs(&self) -> Vec<String>;
    }

    struct StandardFactory;

    impl DecoderFactory for StandardFactory {
        fn create_decoder(&self, codec: &str) -> Option<String> {
            let supported = ["av1", "hevc", "avc"];
            if supported.contains(&codec) {
                Some(format!("{}_decoder", codec))
            } else {
                None
            }
        }

        fn supported_codecs(&self) -> Vec<String> {
            vec!["av1".to_string(), "hevc".to_string(), "avc".to_string()]
        }
    }

    let factory = StandardFactory;
    assert!(factory.create_decoder("av1").is_some());
    assert_eq!(factory.supported_codecs().len(), 3);
}

#[test]
fn test_pixel_format_converter_trait() {
    trait PixelFormatConverter {
        fn convert(&self, src: &[u8], src_format: &str, dst_format: &str) -> Vec<u8>;
        fn supports_conversion(&self, src: &str, dst: &str) -> bool;
    }

    struct SimpleConverter;

    impl PixelFormatConverter for SimpleConverter {
        fn convert(&self, src: &[u8], _src_format: &str, _dst_format: &str) -> Vec<u8> {
            src.to_vec()
        }

        fn supports_conversion(&self, src: &str, dst: &str) -> bool {
            src == "yuv420p" && dst == "rgb24"
        }
    }

    let converter = SimpleConverter;
    assert!(converter.supports_conversion("yuv420p", "rgb24"));
}

#[test]
fn test_frame_callback_trait() {
    trait FrameCallback {
        fn on_frame_decoded(&mut self, frame_index: usize, data: &[u8]);
    }

    struct FrameCounter {
        count: usize,
    }

    impl FrameCallback for FrameCounter {
        fn on_frame_decoded(&mut self, _frame_index: usize, _data: &[u8]) {
            self.count += 1;
        }
    }

    let mut counter = FrameCounter { count: 0 };
    counter.on_frame_decoded(0, &[1, 2, 3]);
    assert_eq!(counter.count, 1);
}

#[test]
fn test_decoder_stats_trait() {
    trait DecoderStats {
        fn frames_decoded(&self) -> usize;
        fn errors_encountered(&self) -> usize;
        fn average_decode_time_ms(&self) -> f64;
    }

    struct SimpleStats {
        frames: usize,
        errors: usize,
        total_time_ms: u64,
    }

    impl DecoderStats for SimpleStats {
        fn frames_decoded(&self) -> usize {
            self.frames
        }

        fn errors_encountered(&self) -> usize {
            self.errors
        }

        fn average_decode_time_ms(&self) -> f64 {
            if self.frames == 0 {
                0.0
            } else {
                self.total_time_ms as f64 / self.frames as f64
            }
        }
    }

    let stats = SimpleStats {
        frames: 10,
        errors: 1,
        total_time_ms: 150,
    };

    assert_eq!(stats.frames_decoded(), 10);
    assert_eq!(stats.average_decode_time_ms(), 15.0);
}

#[test]
fn test_resource_manager_trait() {
    trait ResourceManager {
        fn acquire(&mut self) -> Option<usize>;
        fn release(&mut self, id: usize);
        fn available_count(&self) -> usize;
    }

    struct PoolManager {
        available: Vec<usize>,
        in_use: Vec<usize>,
    }

    impl ResourceManager for PoolManager {
        fn acquire(&mut self) -> Option<usize> {
            if let Some(id) = self.available.pop() {
                self.in_use.push(id);
                Some(id)
            } else {
                None
            }
        }

        fn release(&mut self, id: usize) {
            if let Some(pos) = self.in_use.iter().position(|&x| x == id) {
                self.in_use.remove(pos);
                self.available.push(id);
            }
        }

        fn available_count(&self) -> usize {
            self.available.len()
        }
    }

    let mut manager = PoolManager {
        available: vec![0, 1, 2],
        in_use: vec![],
    };

    let id = manager.acquire().unwrap();
    assert_eq!(manager.available_count(), 2);
    manager.release(id);
    assert_eq!(manager.available_count(), 3);
}
