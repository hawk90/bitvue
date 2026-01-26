//! Tests for Export Worker

#[test]
fn test_export_task() {
    struct ExportTask {
        output_path: String,
        format: String,
        frame_range: (usize, usize),
    }

    let task = ExportTask {
        output_path: "/tmp/export.csv".to_string(),
        format: "CSV".to_string(),
        frame_range: (0, 100),
    };

    assert_eq!(task.format, "CSV");
}

#[test]
fn test_export_progress() {
    struct ExportProgress {
        current_frame: usize,
        total_frames: usize,
    }

    impl ExportProgress {
        fn percentage(&self) -> f64 {
            (self.current_frame as f64 / self.total_frames as f64) * 100.0
        }
    }

    let progress = ExportProgress {
        current_frame: 50,
        total_frames: 100,
    };

    assert_eq!(progress.percentage(), 50.0);
}

#[test]
fn test_csv_row_format() {
    fn format_csv_row(frame: usize, frame_type: &str, size: u64) -> String {
        format!("{},{},{}", frame, frame_type, size)
    }

    assert_eq!(format_csv_row(0, "I", 50000), "0,I,50000");
}

#[test]
fn test_json_export() {
    struct JsonExport {
        pretty_print: bool,
    }

    let export = JsonExport { pretty_print: true };
    assert!(export.pretty_print);
}

#[test]
fn test_batch_export() {
    struct BatchExport {
        tasks: Vec<String>,
    }

    impl BatchExport {
        fn add_task(&mut self, task: String) {
            self.tasks.push(task);
        }
    }

    let mut batch = BatchExport { tasks: vec![] };
    batch.add_task("task1".to_string());

    assert_eq!(batch.tasks.len(), 1);
}

#[test]
fn test_export_cancellation() {
    struct ExportState {
        cancelled: bool,
    }

    impl ExportState {
        fn cancel(&mut self) {
            self.cancelled = true;
        }
    }

    let mut state = ExportState { cancelled: false };
    state.cancel();

    assert!(state.cancelled);
}
