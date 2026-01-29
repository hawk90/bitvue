//! Configuration and persistence methods for BitvueApp

use bitvue_core::BitvueError;
use crate::bitvue_app::BitvueApp;
use crate::panel_tab::PanelTab;
use egui_dock::DockState;

/// Configuration/persistence methods
pub trait BitvueAppConfig {
    fn get_config_dir() -> Result<std::path::PathBuf, BitvueError>;
    fn save_layout(&self) -> Result<(), BitvueError>;
    fn load_layout(&mut self) -> Result<(), BitvueError>;
    fn save_recent_files(&self) -> Result<(), BitvueError>;
    fn load_recent_files(&mut self) -> Result<(), BitvueError>;
}

impl BitvueAppConfig for BitvueApp {
    /// Get config directory path (~/.bitvue/)
    fn get_config_dir() -> Result<std::path::PathBuf, BitvueError> {
        let home_dir =
            dirs::home_dir().ok_or_else(|| BitvueError::InvalidData("Could not determine home directory".to_string()))?;

        let config_dir = home_dir.join(".bitvue");

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir)?;
        }

        Ok(config_dir)
    }

    /// Save current layout to ~/.bitvue/layout.json
    fn save_layout(&self) -> Result<(), BitvueError> {
        let config_dir = Self::get_config_dir()?;
        let layout_path = config_dir.join("layout.json");

        // Serialize DockState to JSON
        let json = serde_json::to_string_pretty(&self.dock_state)?;

        // Write to file
        std::fs::write(&layout_path, json)?;

        tracing::info!("Layout saved to {:?}", layout_path);
        Ok(())
    }

    /// Load layout from ~/.bitvue/layout.json
    fn load_layout(&mut self) -> Result<(), BitvueError> {
        let config_dir = Self::get_config_dir()?;
        let layout_path = config_dir.join("layout.json");

        // Check if file exists
        if !layout_path.exists() {
            return Err(BitvueError::NotFound("No saved layout found".to_string()));
        }

        // Read file
        let json = std::fs::read_to_string(&layout_path)?;

        // Deserialize DockState from JSON
        let dock_state: DockState<PanelTab> = serde_json::from_str(&json)?;

        self.dock_state = dock_state;
        tracing::info!("Layout loaded from {:?}", layout_path);
        Ok(())
    }

    /// Save recent files to ~/.bitvue/recent.json
    fn save_recent_files(&self) -> Result<(), BitvueError> {
        let config_dir = Self::get_config_dir()?;
        let recent_path = config_dir.join("recent.json");

        // Serialize recent files to JSON
        let json = serde_json::to_string_pretty(&self.recent_files)?;

        // Write to file
        std::fs::write(&recent_path, json)?;

        tracing::debug!("Recent files saved to {:?}", recent_path);
        Ok(())
    }

    /// Load recent files from ~/.bitvue/recent.json
    fn load_recent_files(&mut self) -> Result<(), BitvueError> {
        let config_dir = Self::get_config_dir()?;
        let recent_path = config_dir.join("recent.json");

        // Check if file exists
        if !recent_path.exists() {
            tracing::debug!("No recent files found at {:?}", recent_path);
            return Ok(()); // Not an error - just no recent files
        }

        // Read file
        let json = std::fs::read_to_string(&recent_path)?;

        // Deserialize recent files from JSON
        let recent_files: Vec<std::path::PathBuf> = serde_json::from_str(&json)?;

        self.recent_files = recent_files;
        tracing::info!(
            "Loaded {} recent files from {:?}",
            self.recent_files.len(),
            recent_path
        );
        Ok(())
    }
}
