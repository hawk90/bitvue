//! Notification Manager - Extracted from BitvueApp
//!
//! Handles error and success message display with auto-dismiss timeouts.

use std::time::{Duration, Instant};

/// Duration before error messages auto-dismiss
const ERROR_TIMEOUT: Duration = Duration::from_secs(5);

/// Duration before success messages auto-dismiss
const SUCCESS_TIMEOUT: Duration = Duration::from_secs(3);

/// Manages notification display state
#[derive(Debug, Default)]
pub struct NotificationManager {
    /// Error message with timestamp for auto-dismiss
    error_message: Option<(String, Instant)>,
    /// Success message with timestamp for auto-dismiss
    success_message: Option<(String, Instant)>,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an error message to display (auto-dismisses after 5 seconds)
    pub fn set_error(&mut self, message: impl Into<String>) {
        let msg = message.into();
        tracing::error!("{}", msg);
        self.error_message = Some((msg, Instant::now()));
    }

    /// Set a success message to display (auto-dismisses after 3 seconds)
    pub fn set_success(&mut self, message: impl Into<String>) {
        let msg = message.into();
        tracing::info!("{}", msg);
        self.success_message = Some((msg, Instant::now()));
    }

    /// Set a progress message (for async operations)
    ///
    /// This is essentially an alias for set_success() - in the future this
    /// could be enhanced to show a progress bar.
    pub fn set_progress(&mut self, message: impl Into<String>, _progress: f32) {
        self.set_success(message);
    }

    /// Clear error message
    pub fn clear_error(&mut self) {
        self.error_message = None;
    }

    /// Clear success message
    pub fn clear_success(&mut self) {
        self.success_message = None;
    }

    /// Clear all notifications
    pub fn clear_all(&mut self) {
        self.error_message = None;
        self.success_message = None;
    }

    /// Clear notifications if expired
    pub fn check_timeouts(&mut self) {
        if let Some((_, timestamp)) = &self.error_message {
            if timestamp.elapsed() > ERROR_TIMEOUT {
                self.error_message = None;
            }
        }
        if let Some((_, timestamp)) = &self.success_message {
            if timestamp.elapsed() > SUCCESS_TIMEOUT {
                self.success_message = None;
            }
        }
    }

    /// Get current error message if any
    pub fn error(&self) -> Option<&str> {
        self.error_message.as_ref().map(|(msg, _)| msg.as_str())
    }

    /// Get current success message if any
    pub fn success(&self) -> Option<&str> {
        self.success_message.as_ref().map(|(msg, _)| msg.as_str())
    }

    /// Check if there are any active notifications
    pub fn has_notifications(&self) -> bool {
        self.error_message.is_some() || self.success_message.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_has_no_notifications() {
        let mgr = NotificationManager::new();
        assert!(!mgr.has_notifications());
        assert!(mgr.error().is_none());
        assert!(mgr.success().is_none());
    }

    #[test]
    fn test_set_error() {
        let mut mgr = NotificationManager::new();
        mgr.set_error("Test error");
        assert!(mgr.has_notifications());
        assert_eq!(mgr.error(), Some("Test error"));
    }

    #[test]
    fn test_set_success() {
        let mut mgr = NotificationManager::new();
        mgr.set_success("Test success");
        assert!(mgr.has_notifications());
        assert_eq!(mgr.success(), Some("Test success"));
    }

    #[test]
    fn test_clear_error() {
        let mut mgr = NotificationManager::new();
        mgr.set_error("Test error");
        mgr.clear_error();
        assert!(mgr.error().is_none());
    }

    #[test]
    fn test_clear_success() {
        let mut mgr = NotificationManager::new();
        mgr.set_success("Test success");
        mgr.clear_success();
        assert!(mgr.success().is_none());
    }

    #[test]
    fn test_clear_all() {
        let mut mgr = NotificationManager::new();
        mgr.set_error("Error");
        mgr.set_success("Success");
        mgr.clear_all();
        assert!(!mgr.has_notifications());
    }

    #[test]
    fn test_check_timeouts_immediate() {
        let mut mgr = NotificationManager::new();
        mgr.set_error("Error");
        mgr.set_success("Success");

        // Should not clear immediately
        mgr.check_timeouts();
        assert!(mgr.has_notifications());
    }

    #[test]
    fn test_success_timeout() {
        let mut mgr = NotificationManager::new();
        mgr.success_message = Some((
            "Old success".to_string(),
            Instant::now() - Duration::from_secs(4),
        ));

        mgr.check_timeouts();
        assert!(mgr.success().is_none());
    }

    #[test]
    fn test_error_timeout() {
        let mut mgr = NotificationManager::new();
        mgr.error_message = Some((
            "Old error".to_string(),
            Instant::now() - Duration::from_secs(6),
        ));

        mgr.check_timeouts();
        assert!(mgr.error().is_none());
    }
}
