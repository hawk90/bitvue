// Future plugin module tests
//
// Per generate-tests skill: Arrange-Act-Assert pattern with fixtures
// and edge case coverage.

#[allow(unused_imports)]
use super::*;

// ============================================================================
// Fixtures
// ============================================================================

/// Create a test plugin metadata
#[allow(dead_code)]
fn create_test_metadata(id: &str, name: &str) -> PluginMetadata {
    PluginMetadata::new(id.to_string(), name.to_string(), "1.0.0".to_string())
}

/// Create a test plugin registry
#[allow(dead_code)]
fn create_test_registry() -> PluginRegistry {
    PluginRegistry::new()
}

/// Create a test plugin entry
#[allow(dead_code)]
fn create_test_entry(id: &str, name: &str) -> PluginEntry {
    let metadata = create_test_metadata(id, name);
    PluginEntry::new(metadata, 0)
}

// ============================================================================
// PluginMetadata Tests
// ============================================================================

#[allow(unused_must_use, dead_code, unused_imports)]
#[cfg(test)]
mod plugin_metadata_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_metadata() {
        // Arrange & Act
        let metadata = PluginMetadata::new("test_id".to_string(), "Test Plugin".to_string(), "1.0.0".to_string());

        // Assert
        assert_eq!(metadata.id, "test_id");
        assert_eq!(metadata.name, "Test Plugin");
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.author, "");
        assert_eq!(metadata.description, "");
        assert!(metadata.capabilities.is_empty());
        assert_eq!(metadata.required_core_version, ">=0.2.0");
        assert!(metadata.dependencies.is_empty());
    }

    #[test]
    fn test_with_capability_adds() {
        // Arrange
        let metadata = PluginMetadata::new("test_id".to_string(), "Test".to_string(), "1.0.0".to_string());

        // Act
        let metadata = metadata
            .with_capability(PluginCapability::CustomOverlay)
            .with_capability(PluginCapability::CustomMetrics);

        // Assert
        assert_eq!(metadata.capabilities.len(), 2);
        assert!(metadata.capabilities.contains(&PluginCapability::CustomOverlay));
        assert!(metadata.capabilities.contains(&PluginCapability::CustomMetrics));
    }

    #[test]
    fn test_has_capability_true() {
        // Arrange
        let metadata = PluginMetadata::new("test_id".to_string(), "Test".to_string(), "1.0.0".to_string())
            .with_capability(PluginCapability::CustomOverlay);

        // Act
        let has = metadata.has_capability(PluginCapability::CustomOverlay);

        // Assert
        assert!(has);
    }

    #[test]
    fn test_has_capability_false() {
        // Arrange
        let metadata = PluginMetadata::new("test_id".to_string(), "Test".to_string(), "1.0.0".to_string())
            .with_capability(PluginCapability::CustomOverlay);

        // Act
        let has = metadata.has_capability(PluginCapability::CustomExporter);

        // Assert
        assert!(!has);
    }

    #[test]
    fn test_has_capability_empty() {
        // Arrange
        let metadata = PluginMetadata::new("test_id".to_string(), "Test".to_string(), "1.0.0".to_string());

        // Act
        let has = metadata.has_capability(PluginCapability::CustomOverlay);

        // Assert
        assert!(!has);
    }
}

// ============================================================================
// PluginEntry Tests
// ============================================================================

#[allow(unused_must_use, dead_code, unused_imports)]
#[cfg(test)]
mod plugin_entry_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_entry() {
        // Arrange
        let metadata = create_test_metadata("test_id", "Test Plugin");

        // Act
        let entry = PluginEntry::new(metadata, 5);

        // Assert
        assert_eq!(entry.load_order, 5);
        assert_eq!(entry.state, PluginState::Registered);
        assert!(entry.error.is_none());
    }

    #[test]
    fn test_mark_loaded() {
        // Arrange
        let mut entry = create_test_entry("test_id", "Test");

        // Act
        entry.mark_loaded();

        // Assert
        assert_eq!(entry.state, PluginState::Loaded);
        assert!(entry.error.is_none());
    }

    #[test]
    fn test_mark_active() {
        // Arrange
        let mut entry = create_test_entry("test_id", "Test");

        // Act
        entry.mark_active();

        // Assert
        assert_eq!(entry.state, PluginState::Active);
        assert!(entry.error.is_none());
    }

    #[test]
    fn test_mark_failed() {
        // Arrange
        let mut entry = create_test_entry("test_id", "Test");

        // Act
        entry.mark_failed("Test error".to_string());

        // Assert
        assert_eq!(entry.state, PluginState::Failed);
        assert_eq!(entry.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_mark_disabled() {
        // Arrange
        let mut entry = create_test_entry("test_id", "Test");

        // Act
        entry.mark_disabled();

        // Assert
        assert_eq!(entry.state, PluginState::Disabled);
        assert!(entry.error.is_none());
    }

    #[test]
    fn test_mark_failed_clears_on_loaded() {
        // Arrange
        let mut entry = create_test_entry("test_id", "Test");
        entry.mark_failed("Error".to_string());

        // Act
        entry.mark_loaded();

        // Assert
        assert_eq!(entry.state, PluginState::Loaded);
        assert!(entry.error.is_none());
    }
}

// ============================================================================
// PluginRegistry Construction Tests
// ============================================================================

#[allow(unused_must_use, dead_code, unused_imports)]
#[cfg(test)]
mod registry_construction_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_new_creates_registry() {
        // Arrange & Act
        let registry = create_test_registry();

        // Assert
        assert_eq!(registry.count(), 0);
        assert_eq!(registry.next_load_order, 0);
    }

    #[test]
    fn test_default_creates_registry() {
        // Arrange & Act
        let registry = PluginRegistry::default();

        // Assert
        assert_eq!(registry.count(), 0);
    }
}

// ============================================================================
// PluginRegistry Register Tests
// ============================================================================

#[allow(unused_must_use, dead_code, unused_imports)]
#[cfg(test)]
mod registry_register_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_register_adds_plugin() {
        // Arrange
        let mut registry = create_test_registry();
        let metadata = create_test_metadata("plugin1", "Plugin 1");

        // Act
        let result = registry.register(metadata);

        // Assert
        assert!(result.is_ok());
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_register_increments_load_order() {
        // Arrange
        let mut registry = create_test_registry();

        // Act
        registry.register(create_test_metadata("plugin1", "Plugin 1"));
        registry.register(create_test_metadata("plugin2", "Plugin 2"));
        registry.register(create_test_metadata("plugin3", "Plugin 3"));

        // Assert
        let plugins = registry.all();
        assert_eq!(plugins[0].load_order, 0);
        assert_eq!(plugins[1].load_order, 1);
        assert_eq!(plugins[2].load_order, 2);
    }

    #[test]
    fn test_register_duplicate_fails() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));

        // Act
        let result = registry.register(create_test_metadata("plugin1", "Plugin 1 Dup"));

        // Assert
        assert!(result.is_err());
        // Check error contains "already registered"
        match result.unwrap_err() {
            crate::BitvueError::Decode(msg) => assert!(msg.contains("already registered")),
            _ => panic!("Expected Decode error"),
        }
        assert_eq!(registry.count(), 1); // Not added
    }
}

// ============================================================================
// PluginRegistry Unregister Tests
// ============================================================================

#[allow(unused_must_use, dead_code, unused_imports)]
#[cfg(test)]
mod registry_unregister_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_unregister_removes_plugin() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));

        // Act
        let result = registry.unregister("plugin1");

        // Assert
        assert!(result);
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_unregister_nonexistent() {
        // Arrange
        let mut registry = create_test_registry();

        // Act
        let result = registry.unregister("nonexistent");

        // Assert
        assert!(!result);
    }

    #[test]
    fn test_unregister_one_of_many() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));
        registry.register(create_test_metadata("plugin2", "Plugin 2"));
        registry.register(create_test_metadata("plugin3", "Plugin 3"));

        // Act
        registry.unregister("plugin2");

        // Assert
        assert_eq!(registry.count(), 2);
        assert!(registry.get("plugin1").is_some());
        assert!(registry.get("plugin2").is_none());
        assert!(registry.get("plugin3").is_some());
    }
}

// ============================================================================
// PluginRegistry Get Tests
// ============================================================================

#[allow(unused_must_use, dead_code, unused_imports)]
#[cfg(test)]
mod registry_get_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_get_exists() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));

        // Act
        let entry = registry.get("plugin1");

        // Assert
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().metadata.id, "plugin1");
    }

    #[test]
    fn test_get_not_exists() {
        // Arrange
        let registry = create_test_registry();

        // Act
        let entry = registry.get("plugin1");

        // Assert
        assert!(entry.is_none());
    }

    #[test]
    fn test_get_mut_exists() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));

        // Act
        let entry = registry.get_mut("plugin1");

        // Assert
        assert!(entry.is_some());
        entry.unwrap().mark_active();
    }

    #[test]
    fn test_get_mut_not_exists() {
        // Arrange
        let mut registry = create_test_registry();

        // Act
        let entry = registry.get_mut("plugin1");

        // Assert
        assert!(entry.is_none());
    }
}

// ============================================================================
// PluginRegistry Query Tests
// ============================================================================

#[allow(unused_must_use, dead_code, unused_imports)]
#[cfg(test)]
mod registry_query_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_all_returns_sorted_by_load_order() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin3", "Plugin 3"));
        registry.register(create_test_metadata("plugin1", "Plugin 1"));
        registry.register(create_test_metadata("plugin2", "Plugin 2"));

        // Act
        let plugins = registry.all();

        // Assert
        assert_eq!(plugins.len(), 3);
        assert_eq!(plugins[0].metadata.id, "plugin3"); // Registered first (order 0)
        assert_eq!(plugins[1].metadata.id, "plugin1"); // Registered second (order 1)
        assert_eq!(plugins[2].metadata.id, "plugin2"); // Registered third (order 2)
    }

    #[test]
    fn test_by_state() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));
        registry.register(create_test_metadata("plugin2", "Plugin 2"));
        registry.register(create_test_metadata("plugin3", "Plugin 3"));
        registry.mark_loaded("plugin1").unwrap();
        registry.mark_active("plugin2").unwrap();
        // plugin3 stays Registered

        // Act
        let registered = registry.by_state(PluginState::Registered);
        let loaded = registry.by_state(PluginState::Loaded);
        let active = registry.by_state(PluginState::Active);

        // Assert
        assert_eq!(registered.len(), 1);
        assert_eq!(loaded.len(), 1);
        assert_eq!(active.len(), 1);
        assert_eq!(registered[0].metadata.id, "plugin3");
        assert_eq!(loaded[0].metadata.id, "plugin1");
        assert_eq!(active[0].metadata.id, "plugin2");
    }

    #[test]
    fn test_by_capability() {
        // Arrange
        let mut registry = create_test_registry();
        let mut meta1 = create_test_metadata("plugin1", "Plugin 1");
        meta1.capabilities.push(PluginCapability::CustomOverlay);
        let mut meta2 = create_test_metadata("plugin2", "Plugin 2");
        meta2.capabilities.push(PluginCapability::CustomMetrics);
        let mut meta3 = create_test_metadata("plugin3", "Plugin 3");
        meta3.capabilities.push(PluginCapability::CustomOverlay);
        meta3.capabilities.push(PluginCapability::CustomMetrics);

        registry.register(meta1);
        registry.register(meta2);
        registry.register(meta3);

        // Act
        let overlay_plugins = registry.by_capability(PluginCapability::CustomOverlay);
        let metrics_plugins = registry.by_capability(PluginCapability::CustomMetrics);
        let exporter_plugins = registry.by_capability(PluginCapability::CustomExporter);

        // Assert
        assert_eq!(overlay_plugins.len(), 2);
        assert_eq!(metrics_plugins.len(), 2);
        assert_eq!(exporter_plugins.len(), 0);
    }

    #[test]
    fn test_by_capability_empty() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));

        // Act
        let plugins = registry.by_capability(PluginCapability::CustomOverlay);

        // Assert
        assert_eq!(plugins.len(), 0);
    }
}

// ============================================================================
// PluginRegistry State Change Tests
// ============================================================================

#[allow(unused_must_use, dead_code, unused_imports)]
#[cfg(test)]
mod registry_state_change_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_mark_loaded() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));

        // Act
        let result = registry.mark_loaded("plugin1");

        // Assert
        assert!(result.is_ok());
        assert_eq!(registry.get("plugin1").unwrap().state, PluginState::Loaded);
    }

    #[test]
    fn test_mark_loaded_not_found() {
        // Arrange
        let mut registry = create_test_registry();

        // Act
        let result = registry.mark_loaded("plugin1");

        // Assert
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::BitvueError::Decode(msg) => assert!(msg.contains("not found")),
            _ => panic!("Expected Decode error"),
        }
    }

    #[test]
    fn test_mark_active() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));

        // Act
        registry.mark_loaded("plugin1").unwrap();
        let result = registry.mark_active("plugin1");

        // Assert
        assert!(result.is_ok());
        assert_eq!(registry.get("plugin1").unwrap().state, PluginState::Active);
    }

    #[test]
    fn test_mark_failed() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));

        // Act
        let result = registry.mark_failed("plugin1", "Test error".to_string());

        // Assert
        assert!(result.is_ok());
        let entry = registry.get("plugin1").unwrap();
        assert_eq!(entry.state, PluginState::Failed);
        assert_eq!(entry.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_mark_disabled() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));

        // Act
        let result = registry.mark_disabled("plugin1");

        // Assert
        assert!(result.is_ok());
        assert_eq!(registry.get("plugin1").unwrap().state, PluginState::Disabled);
    }

    #[test]
    fn test_state_transitions() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));

        // Act - Full lifecycle
        registry.mark_loaded("plugin1").unwrap();
        assert_eq!(registry.get("plugin1").unwrap().state, PluginState::Loaded);

        registry.mark_active("plugin1").unwrap();
        assert_eq!(registry.get("plugin1").unwrap().state, PluginState::Active);

        registry.mark_disabled("plugin1").unwrap();
        assert_eq!(registry.get("plugin1").unwrap().state, PluginState::Disabled);
    }
}

// ============================================================================
// PluginRegistry Statistics Tests
// ============================================================================

#[allow(unused_must_use, dead_code, unused_imports)]
#[cfg(test)]
mod registry_stats_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_stats_empty() {
        // Arrange
        let registry = create_test_registry();

        // Act
        let stats = registry.stats();

        // Assert
        assert_eq!(stats.total, 0);
        assert_eq!(stats.registered, 0);
        assert_eq!(stats.loaded, 0);
        assert_eq!(stats.active, 0);
        assert_eq!(stats.failed, 0);
        assert_eq!(stats.disabled, 0);
    }

    #[test]
    fn test_stats_counts() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("p1", "P1"));
        registry.register(create_test_metadata("p2", "P2"));
        registry.register(create_test_metadata("p3", "P3"));
        registry.register(create_test_metadata("p4", "P4"));
        registry.register(create_test_metadata("p5", "P5"));

        registry.mark_loaded("p1").unwrap();
        registry.mark_active("p2").unwrap();
        registry.mark_failed("p3", "Error".to_string()).unwrap();
        registry.mark_disabled("p4").unwrap();
        // p5 stays Registered

        // Act
        let stats = registry.stats();

        // Assert
        assert_eq!(stats.total, 5);
        assert_eq!(stats.registered, 1); // p5
        assert_eq!(stats.loaded, 1);     // p1
        assert_eq!(stats.active, 1);     // p2
        assert_eq!(stats.failed, 1);     // p3
        assert_eq!(stats.disabled, 1);   // p4
    }

    #[test]
    fn test_count() {
        // Arrange
        let mut registry = create_test_registry();

        // Act
        registry.register(create_test_metadata("p1", "P1"));
        registry.register(create_test_metadata("p2", "P2"));

        // Assert
        assert_eq!(registry.count(), 2);
    }
}

// ============================================================================
// Edge Cases
// ============================================================================

#[allow(unused_must_use, dead_code, unused_imports)]
#[cfg(test)]
mod edge_case_tests {
#[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_register_after_unregister() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));
        registry.unregister("plugin1");

        // Act - Should be able to re-register
        let result = registry.register(create_test_metadata("plugin1", "Plugin 1 New"));

        // Assert
        assert!(result.is_ok());
        assert_eq!(registry.count(), 1);
    }

    #[test]
    fn test_multiple_plugins_same_capability() {
        // Arrange
        let mut registry = create_test_registry();
        let mut meta1 = create_test_metadata("p1", "P1");
        meta1.capabilities.push(PluginCapability::CustomOverlay);
        let mut meta2 = create_test_metadata("p2", "P2");
        meta2.capabilities.push(PluginCapability::CustomOverlay);

        registry.register(meta1);
        registry.register(meta2);

        // Act
        let plugins = registry.by_capability(PluginCapability::CustomOverlay);

        // Assert
        assert_eq!(plugins.len(), 2);
    }

    #[test]
    fn test_plugin_with_multiple_capabilities() {
        // Arrange
        let mut registry = create_test_registry();
        let mut meta = create_test_metadata("plugin1", "Plugin 1");
        meta.capabilities = vec![
            PluginCapability::CustomOverlay,
            PluginCapability::CustomMetrics,
            PluginCapability::CustomExporter,
        ];

        registry.register(meta);

        // Act
        let overlay = registry.by_capability(PluginCapability::CustomOverlay);
        let metrics = registry.by_capability(PluginCapability::CustomMetrics);
        let exporter = registry.by_capability(PluginCapability::CustomExporter);

        // Assert
        assert_eq!(overlay.len(), 1);
        assert_eq!(metrics.len(), 1);
        assert_eq!(exporter.len(), 1);
        // All should be the same plugin
        assert_eq!(overlay[0].metadata.id, "plugin1");
        assert_eq!(metrics[0].metadata.id, "plugin1");
        assert_eq!(exporter[0].metadata.id, "plugin1");
    }

    #[test]
    fn test_by_state_empty_result() {
        // Arrange
        let mut registry = create_test_registry();
        registry.register(create_test_metadata("plugin1", "Plugin 1"));
        registry.mark_active("plugin1").unwrap();

        // Act
        let failed = registry.by_state(PluginState::Failed);

        // Assert
        assert!(failed.is_empty());
    }

    #[test]
    fn test_mark_state_on_nonexistent() {
        // Arrange
        let mut registry = create_test_registry();

        // Act - All should fail gracefully
        let loaded_result = registry.mark_loaded("nonexistent");
        let active_result = registry.mark_active("nonexistent");
        let failed_result = registry.mark_failed("nonexistent", "Error".to_string());
        let disabled_result = registry.mark_disabled("nonexistent");

        // Assert
        assert!(loaded_result.is_err());
        assert!(active_result.is_err());
        assert!(failed_result.is_err());
        assert!(disabled_result.is_err());
    }

    #[test]
    fn test_get_all_empty_registry() {
        // Arrange
        let registry = create_test_registry();

        // Act
        let plugins = registry.all();

        // Assert
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_metadata_with_all_fields() {
        // Arrange & Act
        let mut metadata = PluginMetadata::new("test_id".to_string(), "Test".to_string(), "2.0.0".to_string());
        metadata.author = "Test Author".to_string();
        metadata.description = "Test Description".to_string();
        metadata.capabilities.push(PluginCapability::CustomOverlay);
        metadata.required_core_version = ">=1.0.0".to_string();
        metadata.dependencies.push("other_plugin".to_string());

        // Assert
        assert_eq!(metadata.author, "Test Author");
        assert_eq!(metadata.description, "Test Description");
        assert_eq!(metadata.required_core_version, ">=1.0.0");
        assert_eq!(metadata.dependencies.len(), 1);
    }
}
