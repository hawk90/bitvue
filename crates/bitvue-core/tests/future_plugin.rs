//! Tests for future_plugin module

use bitvue_core::{PluginCapability, PluginEntry, PluginMetadata, PluginRegistry, PluginState};

#[test]
fn test_plugin_metadata() {
    let metadata = PluginMetadata::new(
        "test.plugin".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
    )
    .with_capability(PluginCapability::CustomOverlay)
    .with_capability(PluginCapability::CustomMetrics);

    assert_eq!(metadata.id, "test.plugin");
    assert_eq!(metadata.name, "Test Plugin");
    assert_eq!(metadata.version, "1.0.0");
    assert!(metadata.has_capability(PluginCapability::CustomOverlay));
    assert!(metadata.has_capability(PluginCapability::CustomMetrics));
    assert!(!metadata.has_capability(PluginCapability::CustomExporter));
}

#[test]
fn test_plugin_entry_states() {
    let metadata = PluginMetadata::new(
        "test.plugin".to_string(),
        "Test Plugin".to_string(),
        "1.0.0".to_string(),
    );

    let mut entry = PluginEntry::new(metadata, 0);
    assert_eq!(entry.state, PluginState::Registered);

    entry.mark_loaded();
    assert_eq!(entry.state, PluginState::Loaded);
    assert!(entry.error.is_none());

    entry.mark_active();
    assert_eq!(entry.state, PluginState::Active);

    entry.mark_failed("Test error".to_string());
    assert_eq!(entry.state, PluginState::Failed);
    assert_eq!(entry.error.as_ref().unwrap(), "Test error");

    entry.mark_disabled();
    assert_eq!(entry.state, PluginState::Disabled);
}

#[test]
fn test_plugin_registry() {
    let mut registry = PluginRegistry::new();

    // Register plugins
    let metadata1 = PluginMetadata::new(
        "plugin1".to_string(),
        "Plugin 1".to_string(),
        "1.0.0".to_string(),
    )
    .with_capability(PluginCapability::CustomOverlay);

    let metadata2 = PluginMetadata::new(
        "plugin2".to_string(),
        "Plugin 2".to_string(),
        "2.0.0".to_string(),
    )
    .with_capability(PluginCapability::CustomMetrics);

    assert!(registry.register(metadata1).is_ok());
    assert!(registry.register(metadata2).is_ok());
    assert_eq!(registry.count(), 2);

    // Try to register duplicate
    let metadata1_dup = PluginMetadata::new(
        "plugin1".to_string(),
        "Plugin 1 Duplicate".to_string(),
        "1.1.0".to_string(),
    );
    assert!(registry.register(metadata1_dup).is_err());

    // Get plugin
    let entry = registry.get("plugin1");
    assert!(entry.is_some());
    assert_eq!(entry.unwrap().metadata.name, "Plugin 1");

    // Mark states
    assert!(registry.mark_loaded("plugin1").is_ok());
    assert!(registry.mark_active("plugin2").is_ok());

    let stats = registry.stats();
    assert_eq!(stats.total, 2);
    assert_eq!(stats.loaded, 1);
    assert_eq!(stats.active, 1);
}

#[test]
fn test_plugin_queries() {
    let mut registry = PluginRegistry::new();

    let metadata1 = PluginMetadata::new(
        "plugin1".to_string(),
        "Plugin 1".to_string(),
        "1.0.0".to_string(),
    )
    .with_capability(PluginCapability::CustomOverlay);

    let metadata2 = PluginMetadata::new(
        "plugin2".to_string(),
        "Plugin 2".to_string(),
        "2.0.0".to_string(),
    )
    .with_capability(PluginCapability::CustomOverlay)
    .with_capability(PluginCapability::CustomMetrics);

    registry.register(metadata1).unwrap();
    registry.register(metadata2).unwrap();

    registry.mark_loaded("plugin1").unwrap();
    registry.mark_active("plugin2").unwrap();

    // Query by state
    let loaded = registry.by_state(PluginState::Loaded);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].metadata.id, "plugin1");

    let active = registry.by_state(PluginState::Active);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].metadata.id, "plugin2");

    // Query by capability
    let overlay_plugins = registry.by_capability(PluginCapability::CustomOverlay);
    assert_eq!(overlay_plugins.len(), 2);

    let metrics_plugins = registry.by_capability(PluginCapability::CustomMetrics);
    assert_eq!(metrics_plugins.len(), 1);
    assert_eq!(metrics_plugins[0].metadata.id, "plugin2");
}

#[test]
fn test_plugin_unregister() {
    let mut registry = PluginRegistry::new();

    let metadata = PluginMetadata::new(
        "plugin1".to_string(),
        "Plugin 1".to_string(),
        "1.0.0".to_string(),
    );

    registry.register(metadata).unwrap();
    assert_eq!(registry.count(), 1);

    assert!(registry.unregister("plugin1"));
    assert_eq!(registry.count(), 0);

    assert!(!registry.unregister("plugin1"));
}
