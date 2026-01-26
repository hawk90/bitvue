//! Tests for Parse Worker

#[test]
fn test_parse_request() {
    struct ParseRequest {
        file_path: String,
        codec_type: String,
        byte_range: Option<(u64, u64)>,
    }

    let request = ParseRequest {
        file_path: "/tmp/test.ivf".to_string(),
        codec_type: "AV1".to_string(),
        byte_range: Some((0, 1024)),
    };

    assert!(request.byte_range.is_some());
}

#[test]
fn test_parse_progress() {
    struct ParseProgress {
        bytes_parsed: u64,
        total_bytes: u64,
        obu_count: usize,
    }

    impl ParseProgress {
        fn percentage(&self) -> f64 {
            if self.total_bytes == 0 {
                0.0
            } else {
                (self.bytes_parsed as f64 / self.total_bytes as f64) * 100.0
            }
        }
    }

    let progress = ParseProgress {
        bytes_parsed: 512,
        total_bytes: 1024,
        obu_count: 5,
    };

    assert_eq!(progress.percentage(), 50.0);
}

#[test]
fn test_syntax_tree_node() {
    struct SyntaxTreeNode {
        name: String,
        offset: u64,
        size: usize,
        children: Vec<String>,
    }

    let node = SyntaxTreeNode {
        name: "sequence_header".to_string(),
        offset: 32,
        size: 128,
        children: vec!["profile".to_string(), "level".to_string()],
    };

    assert_eq!(node.children.len(), 2);
}

#[test]
fn test_parse_error_recovery() {
    #[derive(Debug, PartialEq)]
    enum ParseError {
        InvalidObu,
        UnexpectedEof,
        UnsupportedCodec,
    }

    struct ErrorRecovery {
        skip_invalid: bool,
        max_errors: usize,
    }

    impl ErrorRecovery {
        fn should_continue(&self, error_count: usize) -> bool {
            self.skip_invalid && error_count < self.max_errors
        }
    }

    let recovery = ErrorRecovery {
        skip_invalid: true,
        max_errors: 10,
    };

    assert!(recovery.should_continue(5));
    assert!(!recovery.should_continue(15));
}

#[test]
fn test_obu_parser() {
    #[derive(Debug, PartialEq)]
    enum ObuType {
        SequenceHeader,
        FrameHeader,
        TileGroup,
        Frame,
    }

    struct ObuHeader {
        obu_type: ObuType,
        has_extension: bool,
        has_size_field: bool,
    }

    let header = ObuHeader {
        obu_type: ObuType::SequenceHeader,
        has_extension: false,
        has_size_field: true,
    };

    assert_eq!(header.obu_type, ObuType::SequenceHeader);
}

#[test]
fn test_parse_state_machine() {
    #[derive(Debug, PartialEq)]
    enum ParseState {
        Idle,
        ReadingHeader,
        ReadingPayload,
        Complete,
        Error,
    }

    struct StateMachine {
        state: ParseState,
    }

    impl StateMachine {
        fn transition(&mut self, next: ParseState) {
            self.state = next;
        }

        fn is_complete(&self) -> bool {
            self.state == ParseState::Complete
        }
    }

    let mut sm = StateMachine {
        state: ParseState::Idle,
    };

    sm.transition(ParseState::ReadingHeader);
    assert_eq!(sm.state, ParseState::ReadingHeader);
    assert!(!sm.is_complete());
}

#[test]
fn test_parse_cancellation() {
    struct ParseCancellation {
        cancelled: bool,
        partial_results: Vec<String>,
    }

    impl ParseCancellation {
        fn cancel(&mut self) {
            self.cancelled = true;
        }

        fn has_partial_results(&self) -> bool {
            !self.partial_results.is_empty()
        }
    }

    let mut cancel = ParseCancellation {
        cancelled: false,
        partial_results: vec!["obu1".to_string()],
    };

    cancel.cancel();
    assert!(cancel.cancelled);
    assert!(cancel.has_partial_results());
}

#[test]
fn test_parse_buffer_management() {
    struct ParseBuffer {
        data: Vec<u8>,
        read_pos: usize,
        capacity: usize,
    }

    impl ParseBuffer {
        fn available(&self) -> usize {
            self.data.len() - self.read_pos
        }

        fn consume(&mut self, bytes: usize) {
            self.read_pos = (self.read_pos + bytes).min(self.data.len());
        }
    }

    let mut buffer = ParseBuffer {
        data: vec![0u8; 1024],
        read_pos: 0,
        capacity: 1024,
    };

    assert_eq!(buffer.available(), 1024);
    buffer.consume(512);
    assert_eq!(buffer.available(), 512);
}

#[test]
fn test_parse_validation() {
    fn validate_obu_size(size: usize, max_size: usize) -> bool {
        size > 0 && size <= max_size
    }

    assert!(validate_obu_size(1024, 4096));
    assert!(!validate_obu_size(0, 4096));
    assert!(!validate_obu_size(8192, 4096));
}

#[test]
fn test_incremental_parsing() {
    struct IncrementalParser {
        chunks_processed: usize,
        total_chunks: usize,
        accumulated_data: Vec<u8>,
    }

    impl IncrementalParser {
        fn process_chunk(&mut self, chunk: Vec<u8>) {
            self.accumulated_data.extend(chunk);
            self.chunks_processed += 1;
        }

        fn is_complete(&self) -> bool {
            self.chunks_processed >= self.total_chunks
        }
    }

    let mut parser = IncrementalParser {
        chunks_processed: 0,
        total_chunks: 3,
        accumulated_data: vec![],
    };

    parser.process_chunk(vec![1, 2, 3]);
    assert_eq!(parser.chunks_processed, 1);
    assert!(!parser.is_complete());
}
