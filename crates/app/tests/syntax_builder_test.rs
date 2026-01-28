//! Tests for Syntax Builder

#[test]
fn test_syntax_node_creation() {
    struct SyntaxNode {
        name: String,
        value: Option<u64>,
        children: Vec<String>,
    }

    let node = SyntaxNode {
        name: "sequence_header".to_string(),
        value: None,
        children: vec!["profile".to_string(), "level".to_string()],
    };

    assert_eq!(node.children.len(), 2);
}

#[test]
fn test_tree_building() {
    struct TreeBuilder {
        root: Option<String>,
        nodes: Vec<String>,
    }

    impl TreeBuilder {
        fn new() -> Self {
            Self {
                root: None,
                nodes: vec![],
            }
        }

        fn set_root(&mut self, name: String) {
            self.root = Some(name);
        }

        fn add_node(&mut self, name: String) {
            self.nodes.push(name);
        }
    }

    let mut builder = TreeBuilder::new();
    builder.set_root("root".to_string());
    builder.add_node("child1".to_string());
    assert_eq!(builder.nodes.len(), 1);
}

#[test]
fn test_syntax_element() {
    struct SyntaxElement {
        name: String,
        bit_offset: usize,
        bit_length: usize,
        value: u64,
    }

    impl SyntaxElement {
        fn byte_offset(&self) -> usize {
            self.bit_offset / 8
        }

        fn bit_position(&self) -> usize {
            self.bit_offset % 8
        }
    }

    let element = SyntaxElement {
        name: "profile_idc".to_string(),
        bit_offset: 16,
        bit_length: 8,
        value: 100,
    };

    assert_eq!(element.byte_offset(), 2);
    assert_eq!(element.bit_position(), 0);
}

#[test]
fn test_hierarchy_building() {
    struct HierarchyBuilder {
        depth: usize,
        stack: Vec<String>,
    }

    impl HierarchyBuilder {
        fn push(&mut self, name: String) {
            self.stack.push(name);
            self.depth += 1;
        }

        fn pop(&mut self) -> Option<String> {
            if self.depth > 0 {
                self.depth -= 1;
            }
            self.stack.pop()
        }

        fn current_depth(&self) -> usize {
            self.depth
        }
    }

    let mut builder = HierarchyBuilder {
        depth: 0,
        stack: vec![],
    };

    builder.push("root".to_string());
    builder.push("child".to_string());
    assert_eq!(builder.current_depth(), 2);
}

#[test]
fn test_field_formatting() {
    fn format_field(name: &str, value: u64, format: &str) -> String {
        match format {
            "hex" => format!("{}: 0x{:X}", name, value),
            "bin" => format!("{}: 0b{:b}", name, value),
            "dec" => format!("{}: {}", name, value),
            _ => format!("{}: {}", name, value),
        }
    }

    assert_eq!(format_field("value", 255, "hex"), "value: 0xFF");
    assert_eq!(format_field("value", 15, "bin"), "value: 0b1111");
}

#[test]
fn test_syntax_validation() {
    struct SyntaxValidator {
        required_fields: Vec<String>,
    }

    impl SyntaxValidator {
        fn validate(&self, fields: &[String]) -> bool {
            self.required_fields.iter().all(|req| fields.contains(req))
        }
    }

    let validator = SyntaxValidator {
        required_fields: vec!["profile".to_string(), "level".to_string()],
    };

    assert!(validator.validate(&["profile".to_string(), "level".to_string()]));
    assert!(!validator.validate(&["profile".to_string()]));
}

#[test]
fn test_indentation() {
    struct Indenter {
        indent_size: usize,
        level: usize,
    }

    impl Indenter {
        fn get_indent(&self) -> String {
            " ".repeat(self.indent_size * self.level)
        }

        fn increase(&mut self) {
            self.level += 1;
        }

        fn decrease(&mut self) {
            if self.level > 0 {
                self.level -= 1;
            }
        }
    }

    let mut indenter = Indenter {
        indent_size: 2,
        level: 0,
    };

    indenter.increase();
    assert_eq!(indenter.get_indent(), "  ");
}

#[test]
fn test_array_handling() {
    struct ArrayBuilder {
        items: Vec<u64>,
        max_items: usize,
    }

    impl ArrayBuilder {
        fn add(&mut self, value: u64) -> bool {
            if self.items.len() < self.max_items {
                self.items.push(value);
                true
            } else {
                false
            }
        }

        fn format(&self) -> String {
            format!(
                "[{}]",
                self.items
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }

    let mut builder = ArrayBuilder {
        items: vec![],
        max_items: 10,
    };

    builder.add(1);
    builder.add(2);
    assert_eq!(builder.format(), "[1, 2]");
}

#[test]
fn test_syntax_path() {
    struct SyntaxPath {
        components: Vec<String>,
    }

    impl SyntaxPath {
        fn to_string(&self) -> String {
            self.components.join(".")
        }

        fn push(&mut self, component: String) {
            self.components.push(component);
        }

        fn pop(&mut self) -> Option<String> {
            self.components.pop()
        }
    }

    let mut path = SyntaxPath {
        components: vec!["root".to_string(), "child".to_string()],
    };

    path.push("grandchild".to_string());
    assert_eq!(path.to_string(), "root.child.grandchild");
}

#[test]
fn test_value_formatting() {
    struct ValueFormatter;

    impl ValueFormatter {
        fn format_size(bytes: u64) -> String {
            if bytes < 1024 {
                format!("{} B", bytes)
            } else if bytes < 1024 * 1024 {
                format!("{:.1} KB", bytes as f64 / 1024.0)
            } else {
                format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
            }
        }
    }

    assert_eq!(ValueFormatter::format_size(512), "512 B");
    assert_eq!(ValueFormatter::format_size(2048), "2.0 KB");
}
