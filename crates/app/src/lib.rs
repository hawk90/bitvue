//! bitvue app library
//!
//! Exposes parser_worker and other modules for testing

// BitvueApp modules
pub mod app_async;
pub mod app_config;
pub mod app_decode;
pub mod app_input;
pub mod app_ui_menus;
pub mod app_ui_panels;
pub mod app_ui_toolbar;
pub mod app_update;
pub mod app_yuv_diff;
pub mod bitvue_app;

// Other app modules
pub mod app_ui;
pub mod bytecache_worker;
pub mod config_worker;
pub mod decode_coordinator;
pub mod decode_worker;
pub mod export;
pub mod export_worker;
pub mod file_ops;
pub mod helpers;
pub mod lazy_workspace;
pub mod notifications;
pub mod panel_registry;
pub mod panel_tab;
pub mod panel_tab_viewer;
pub mod parse_coordinator;
pub mod parse_worker;
pub mod parser_worker;
pub mod retry_policy;
pub mod settings;
pub mod syntax_builder;
pub mod workspace_registry;
pub mod yuv_diff;
