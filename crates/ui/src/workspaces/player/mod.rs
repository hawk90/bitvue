//! Player Workspace Modules
//!
//! Decomposed player workspace functionality into focused modules:
//! - texture: Frame texture management
//! - navigation: Frame navigation and keyboard shortcuts
//! - zoom: Zoom and pan state management
//! - partition_loader: Partition data loading

mod navigation;
mod partition_loader;
mod texture;
mod zoom;

pub use navigation::NavigationManager;
pub use partition_loader::PartitionLoader;
pub use texture::TextureManager;
pub use zoom::ZoomManager;
