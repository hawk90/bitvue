//! Configuration and persistence methods for BitvueApp

use crate::bitvue_app::BitvueApp;
use crate::panel_tab::PanelTab;
use egui_dock::DockState;

/// Configuration/persistence methods
pub trait BitvueAppConfig {
    fn get_config_dir() -> Result<std::path::PathBuf, String>;
    fn save_layout(&self) -> Result<(), String>;
    fn load_layout(&mut self) -> Result<(), String>;
    fn save_recent_files(&self) -> Result<(), String>;
    fn load_recent_files(&mut self) -> Result<(), String>;
}

impl BitvueAppConfig for BitvueApp {
    /// Get config directory path (~/.bitvue/)
    fn get_config_dir() -> Result<std::path::PathBuf, String> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| "Could not determine home directory".to_string())?;

        let config_dir = home_dir.join(".bitvue");

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)
                .map_err(|e| format!("Failed to create config directory: {}", e))?;
        }

        Ok(config_dir)
    }

    /// Save current layout to ~/.bitvue/layout.json
    fn save_layout(&self) -> Result<(), String> {
        let config_dir = Self::get_config_dir()?;
        let layout_path = config_dir.join("layout.json");

        // Serialize DockState to JSON
        let json = serde_json::to_string_pretty(&self.dock_state)
            .map_err(|e| format!("Failed to serialize layout: {}", e))?;

        // Write to file
        std::fs::write(&layout_path, json)
            .map_err(|e| format!("Failed to write layout file: {}", e))?;

        tracing::info!("Layout saved to {:?}", layout_path);
        Ok(())
    }

    /// Load layout from ~/.bitvue/layout.json
    fn load_layout(&mut self) -> Result<(), String> {
        let config_dir = Self::get_config_dir()?;
        let layout_path = config_dir.join("layout.json");

        // Check if file exists
        if !layout_path.exists() {
            return Err("No saved layout found".to_string());
        }

        // Read file
        let json = std::fs::read_to_string(&layout_path)
            .map_err(|e| format!("Failed to read layout file: {}", e))?;

        // Deserialize DockState from JSON
        let dock_state: DockState<PanelTab> = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize layout: {}", e))?;

        self.dock_state = dock_state;
        tracing::info!("Layout loaded from {:?}", layout_path);
        Ok(())
    }

    /// Save recent files to ~/.bitvue/recent.json
    fn save_recent_files(&self) -> Result<(), String> {
        let config_dir = Self::get_config_dir()?;
        let recent_path = config_dir.join("recent.json");

        // Serialize recent files to JSON
        let json = serde_json::to_string_pretty(&self.recent_files)
            .map_err(|e| format!("Failed to serialize recent files: {}", e))?;

        // Write to file
        std::fs::write(&recent_path, json)
            .map_err(|e| format!("Failed to write recent files: {}", e))?;

        tracing::debug!("Recent files saved to {:?}", recent_path);
        Ok(())
    }

    /// Load recent files from ~/.bitvue/recent.json
    fn load_recent_files(&mut self) -> Result<(), String> {
        let config_dir = Self::get_config_dir()?;
        let recent_path = config_dir.join("recent.json");

        // Check if file exists
        if !recent_path.exists() {
            tracing::debug!("No recent files found at {:?}", recent_path);
            return Ok(()); // Not an error - just no recent files
        }

        // Read file
        let json = std::fs::read_to_string(&recent_path)
            .map_err(|e| format!("Failed to read recent files: {}", e))?;

        // Deserialize recent files from JSON
        let recent_files: Vec<std::path::PathBuf> = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to deserialize recent files: {}", e))?;

        self.recent_files = recent_files;
        tracing::info!(
            "Loaded {} recent files from {:?}",
            self.recent_files.len(),
            recent_path
        );
        Ok(())
    }
}
