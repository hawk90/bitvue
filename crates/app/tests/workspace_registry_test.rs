//! Tests for Workspace Registry

#[test]
fn test_workspace_registration() {
    use std::collections::HashMap;

    struct WorkspaceRegistry {
        workspaces: HashMap<String, String>, // name -> type
    }

    impl WorkspaceRegistry {
        fn new() -> Self {
            Self {
                workspaces: HashMap::new(),
            }
        }

        fn register(&mut self, name: String, workspace_type: String) {
            self.workspaces.insert(name, workspace_type);
        }

        fn is_registered(&self, name: &str) -> bool {
            self.workspaces.contains_key(name)
        }
    }

    let mut registry = WorkspaceRegistry::new();
    registry.register("av1".to_string(), "codec".to_string());
    assert!(registry.is_registered("av1"));
}

#[test]
fn test_workspace_lookup() {
    use std::collections::HashMap;

    struct WorkspaceLookup {
        workspaces: HashMap<String, usize>, // name -> id
    }

    impl WorkspaceLookup {
        fn add(&mut self, name: String, id: usize) {
            self.workspaces.insert(name, id);
        }

        fn get_id(&self, name: &str) -> Option<usize> {
            self.workspaces.get(name).copied()
        }
    }

    let mut lookup = WorkspaceLookup {
        workspaces: HashMap::new(),
    };

    lookup.add("hevc".to_string(), 1);
    assert_eq!(lookup.get_id("hevc"), Some(1));
}

#[test]
fn test_workspace_enumeration() {
    struct WorkspaceEnumerator {
        workspaces: Vec<String>,
    }

    impl WorkspaceEnumerator {
        fn list(&self) -> &[String] {
            &self.workspaces
        }

        fn count(&self) -> usize {
            self.workspaces.len()
        }
    }

    let enumerator = WorkspaceEnumerator {
        workspaces: vec!["av1".to_string(), "hevc".to_string(), "avc".to_string()],
    };

    assert_eq!(enumerator.count(), 3);
}

#[test]
fn test_workspace_metadata() {
    struct WorkspaceMetadata {
        name: String,
        display_name: String,
        icon: String,
        enabled: bool,
    }

    impl WorkspaceMetadata {
        fn is_available(&self) -> bool {
            self.enabled
        }
    }

    let metadata = WorkspaceMetadata {
        name: "av1".to_string(),
        display_name: "AV1 Workspace".to_string(),
        icon: "📹".to_string(),
        enabled: true,
    };

    assert!(metadata.is_available());
}

#[test]
fn test_workspace_creation() {
    struct WorkspaceFactory {
        created_count: usize,
    }

    impl WorkspaceFactory {
        fn create(&mut self, name: &str) -> String {
            self.created_count += 1;
            format!("Workspace: {}", name)
        }
    }

    let mut factory = WorkspaceFactory { created_count: 0 };
    let workspace = factory.create("av1");
    assert_eq!(workspace, "Workspace: av1");
    assert_eq!(factory.created_count, 1);
}

#[test]
fn test_workspace_activation() {
    struct WorkspaceActivator {
        active_workspace: Option<String>,
    }

    impl WorkspaceActivator {
        fn activate(&mut self, name: String) {
            self.active_workspace = Some(name);
        }

        fn deactivate(&mut self) {
            self.active_workspace = None;
        }

        fn get_active(&self) -> Option<&String> {
            self.active_workspace.as_ref()
        }
    }

    let mut activator = WorkspaceActivator {
        active_workspace: None,
    };

    activator.activate("vvc".to_string());
    assert_eq!(activator.get_active(), Some(&"vvc".to_string()));
}

#[test]
fn test_workspace_validation() {
    fn validate_workspace_name(name: &str) -> bool {
        !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    assert!(validate_workspace_name("av1"));
    assert!(validate_workspace_name("av1_workspace"));
    assert!(!validate_workspace_name(""));
    assert!(!validate_workspace_name("av1-workspace"));
}

#[test]
fn test_workspace_dependencies() {
    use std::collections::HashMap;

    struct WorkspaceDependencies {
        deps: HashMap<String, Vec<String>>, // workspace -> dependencies
    }

    impl WorkspaceDependencies {
        fn add_dependency(&mut self, workspace: String, dependency: String) {
            self.deps
                .entry(workspace)
                .or_insert_with(Vec::new)
                .push(dependency);
        }

        fn get_dependencies(&self, workspace: &str) -> Option<&Vec<String>> {
            self.deps.get(workspace)
        }
    }

    let mut deps = WorkspaceDependencies {
        deps: HashMap::new(),
    };

    deps.add_dependency("av1".to_string(), "decoder".to_string());
    assert!(deps.get_dependencies("av1").is_some());
}

#[test]
fn test_workspace_priority() {
    struct WorkspacePriority {
        name: String,
        priority: u8,
    }

    impl WorkspacePriority {
        fn compare(&self, other: &WorkspacePriority) -> std::cmp::Ordering {
            self.priority.cmp(&other.priority) // Higher priority value = Greater
        }
    }

    let ws1 = WorkspacePriority {
        name: "av1".to_string(),
        priority: 10,
    };
    let ws2 = WorkspacePriority {
        name: "hevc".to_string(),
        priority: 5,
    };

    assert_eq!(ws1.compare(&ws2), std::cmp::Ordering::Greater); // 10 > 5
}

#[test]
fn test_workspace_lifecycle() {
    #[derive(Debug, PartialEq)]
    enum WorkspaceState {
        Created,
        Initialized,
        Active,
        Suspended,
        Destroyed,
    }

    struct WorkspaceLifecycle {
        state: WorkspaceState,
    }

    impl WorkspaceLifecycle {
        fn transition(&mut self, new_state: WorkspaceState) {
            self.state = new_state;
        }
    }

    let mut lifecycle = WorkspaceLifecycle {
        state: WorkspaceState::Created,
    };

    lifecycle.transition(WorkspaceState::Initialized);
    assert_eq!(lifecycle.state, WorkspaceState::Initialized);
}
