//! Status annotations for adding metadata to error statuses.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::status::Status;

/// Annotation for a status that provides additional context.
#[derive(Clone, Debug, PartialEq)]
pub struct StatusAnnotation {
    /// Key for the annotation.
    pub key: String,
    /// Value for the annotation.
    pub value: String,
}

impl StatusAnnotation {
    /// Creates a new status annotation.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }

    /// Creates a "file" annotation.
    pub fn file(file: impl Into<String>) -> Self {
        Self::new("file", file)
    }

    /// Creates a "line" annotation.
    pub fn line(line: u32) -> Self {
        Self::new("line", line.to_string())
    }

    /// Creates a "function" annotation.
    pub fn function(func: impl Into<String>) -> Self {
        Self::new("function", func)
    }

    /// Creates a "module" annotation.
    pub fn module(module: impl Into<String>) -> Self {
        Self::new("module", module)
    }
}

/// Extended status with annotations.
#[derive(Clone, Debug)]
pub struct AnnotatedStatus {
    /// The underlying status.
    status: Status,
    /// Annotations attached to this status.
    annotations: Vec<StatusAnnotation>,
}

impl AnnotatedStatus {
    /// Creates a new annotated status.
    pub fn new(status: Status) -> Self {
        Self {
            status,
            annotations: Vec::new(),
        }
    }

    /// Adds an annotation.
    pub fn with_annotation(mut self, annotation: StatusAnnotation) -> Self {
        self.annotations.push(annotation);
        self
    }

    /// Adds a file:line annotation.
    pub fn with_location(mut self, file: &str, line: u32) -> Self {
        self.annotations.push(StatusAnnotation::file(file));
        self.annotations.push(StatusAnnotation::line(line));
        self
    }

    /// Adds a function annotation.
    pub fn with_function(mut self, func: &str) -> Self {
        self.annotations.push(StatusAnnotation::function(func));
        self
    }

    /// Returns the underlying status.
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Returns the annotations.
    pub fn annotations(&self) -> &[StatusAnnotation] {
        &self.annotations
    }

    /// Converts to a regular Status with annotations in the message.
    pub fn to_status(&self) -> Status {
        if self.annotations.is_empty() {
            return self.status.clone();
        }

        let annotated_msg = format!(
            "{}\nAnnotations: {}",
            self.status.message(),
            self.annotations
                .iter()
                .map(|a| format!("{}={}", a.key, a.value))
                .collect::<Vec<_>>()
                .join(", ")
        );

        Status::new(self.status.code(), annotated_msg)
    }
}

impl From<Status> for AnnotatedStatus {
    fn from(status: Status) -> Self {
        Self::new(status)
    }
}

impl From<AnnotatedStatus> for Status {
    fn from(annotated: AnnotatedStatus) -> Self {
        annotated.to_status()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // StatusAnnotation tests
    #[test]
    fn test_status_annotation_new() {
        let ann = StatusAnnotation::new("key", "value");
        assert_eq!(ann.key, "key");
        assert_eq!(ann.value, "value");
    }

    #[test]
    fn test_status_annotation_helpers() {
        let file_ann = StatusAnnotation::file("test.rs");
        assert_eq!(file_ann.key, "file");
        assert_eq!(file_ann.value, "test.rs");

        let line_ann = StatusAnnotation::line(42);
        assert_eq!(line_ann.key, "line");
        assert_eq!(line_ann.value, "42");

        let func_ann = StatusAnnotation::function("test_func");
        assert_eq!(func_ann.key, "function");
        assert_eq!(func_ann.value, "test_func");

        let mod_ann = StatusAnnotation::module("test_module");
        assert_eq!(mod_ann.key, "module");
        assert_eq!(mod_ann.value, "test_module");
    }

    // AnnotatedStatus tests
    #[test]
    fn test_annotated_status_new() {
        let status = Status::new(StatusCode::Internal, "Error");
        let annotated = AnnotatedStatus::new(status);
        assert_eq!(annotated.status().code(), StatusCode::Internal);
    }

    #[test]
    fn test_annotated_status_with_annotation() {
        let status = Status::new(StatusCode::Internal, "Error");
        let annotated = AnnotatedStatus::new(status)
            .with_annotation(StatusAnnotation::file("test.rs"))
            .with_annotation(StatusAnnotation::line(42));

        assert_eq!(annotated.annotations().len(), 2);
    }

    #[test]
    fn test_annotated_status_with_location() {
        let status = Status::new(StatusCode::Internal, "Error");
        let annotated = AnnotatedStatus::new(status)
            .with_location("test.rs", 42);

        assert_eq!(annotated.annotations().len(), 2);
    }

    #[test]
    fn test_annotated_status_with_function() {
        let status = Status::new(StatusCode::Internal, "Error");
        let annotated = AnnotatedStatus::new(status)
            .with_function("test_func");

        assert_eq!(annotated.annotations().len(), 1);
        assert_eq!(annotated.annotations()[0].key, "function");
    }

    #[test]
    fn test_annotated_status_to_status() {
        let status = Status::new(StatusCode::Internal, "Error");
        let annotated = AnnotatedStatus::new(status)
            .with_location("test.rs", 42);

        let converted = annotated.to_status();
        assert!(converted.message().contains("test.rs"));
    }

    #[test]
    fn test_annotated_status_from_status() {
        let status = Status::new(StatusCode::Internal, "Error");
        let annotated: AnnotatedStatus = status.clone().into();
        assert_eq!(annotated.status().code(), StatusCode::Internal);
    }
}
