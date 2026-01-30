/**
 * Log commands for forwarding frontend logs to terminal
 */

use tauri::command;

#[command]
pub fn frontend_log(level: String, message: String) {
    match level.as_str() {
        "log" | "info" => println!("[FRONTEND] {}", message),
        "warn" => println!("[FRONTEND] WARN: {}", message),
        "error" => eprintln!("[FRONTEND] ERROR: {}", message),
        "debug" => println!("[FRONTEND] DEBUG: {}", message),
        _ => println!("[FRONTEND] {}", message),
    }
}
