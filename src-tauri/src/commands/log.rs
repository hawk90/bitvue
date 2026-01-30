/**
 * Log commands for forwarding frontend logs to terminal
 *
 * SECURITY: Sanitizes frontend logs to prevent log injection attacks.
 * - Removes control characters (newlines, tabs, etc.)
 * - Limits message length to prevent DoS
 * - Validates log level to prevent arbitrary log level injection
 */

use tauri::command;

/// Maximum log message length to prevent DoS through oversized logs
const MAX_LOG_LENGTH: usize = 4096;

/// Characters to sanitize (control characters that could be used for injection)
const SANITIZE_CHARS: &[char] = &['\r', '\n', '\t', '\x00', '\x1b']; // ESC for ANSI escape sequences

/// Sanitize a log message by removing dangerous characters
fn sanitize_log_message(message: &str) -> String {
    let mut sanitized = String::with_capacity(message.len().min(MAX_LOG_LENGTH));

    for ch in message.chars().take(MAX_LOG_LENGTH) {
        if SANITIZE_CHARS.contains(&ch) {
            // Replace with space
            sanitized.push(' ');
        } else {
            sanitized.push(ch);
        }
    }

    // Also truncate if we took the first MAX_LOG_LENGTH chars
    if message.len() > MAX_LOG_LENGTH {
        sanitized.push_str("...");
    }

    sanitized
}

/// Validate log level to prevent injection
fn validate_log_level(level: &str) -> bool {
    matches!(level, "log" | "info" | "warn" | "error" | "debug")
}

#[command]
pub fn frontend_log(level: String, message: String) {
    // Validate log level first
    if !validate_log_level(&level) {
        // Don't log arbitrary levels, use a default
        println!("[FRONTEND] [INVALID_LEVEL] {}", sanitize_log_message(&message));
        return;
    }

    // Sanitize the message to prevent injection
    let sanitized = sanitize_log_message(&message);

    match level.as_str() {
        "log" | "info" => println!("[FRONTEND] {}", sanitized),
        "warn" => println!("[FRONTEND] WARN: {}", sanitized),
        "error" => eprintln!("[FRONTEND] ERROR: {}", sanitized),
        "debug" => println!("[FRONTEND] DEBUG: {}", sanitized),
        _ => println!("[FRONTEND] {}", sanitized),
    }
}
