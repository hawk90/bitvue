//! Tests for App UI Menus

#[test]
fn test_menu_item() {
    struct MenuItem {
        label: String,
        shortcut: Option<String>,
        enabled: bool,
        separator: bool,
    }

    let item = MenuItem {
        label: "Open File".to_string(),
        shortcut: Some("Ctrl+O".to_string()),
        enabled: true,
        separator: false,
    };

    assert!(item.enabled);
    assert!(!item.separator);
}

#[test]
fn test_file_menu() {
    struct FileMenu {
        recent_files: Vec<String>,
        max_recent: usize,
    }

    impl FileMenu {
        fn add_recent(&mut self, path: String) {
            if let Some(pos) = self.recent_files.iter().position(|p| p == &path) {
                self.recent_files.remove(pos);
            }
            self.recent_files.insert(0, path);
            if self.recent_files.len() > self.max_recent {
                self.recent_files.truncate(self.max_recent);
            }
        }

        fn clear_recent(&mut self) {
            self.recent_files.clear();
        }
    }

    let mut menu = FileMenu {
        recent_files: vec![],
        max_recent: 9,
    };

    menu.add_recent("/tmp/file1.ivf".to_string());
    assert_eq!(menu.recent_files.len(), 1);
    menu.clear_recent();
    assert!(menu.recent_files.is_empty());
}

#[test]
fn test_mode_menu() {
    #[derive(Debug, PartialEq, Clone)]
    enum CodecMode {
        Av1,
        Hevc,
        Avc,
        Vvc,
        Vp9,
        Mpeg2,
    }

    struct ModeMenu {
        current_mode: CodecMode,
        available_modes: Vec<CodecMode>,
    }

    impl ModeMenu {
        fn switch_mode(&mut self, mode: CodecMode) {
            if self.available_modes.contains(&mode) {
                self.current_mode = mode;
            }
        }
    }

    let mut menu = ModeMenu {
        current_mode: CodecMode::Av1,
        available_modes: vec![CodecMode::Av1, CodecMode::Hevc],
    };

    menu.switch_mode(CodecMode::Hevc);
    assert_eq!(menu.current_mode, CodecMode::Hevc);
}

#[test]
fn test_yuv_diff_menu() {
    struct YuvDiffMenu {
        enabled: bool,
        reference_path: Option<String>,
        diff_mode: String,
    }

    impl YuvDiffMenu {
        fn set_reference(&mut self, path: String) {
            self.reference_path = Some(path);
            self.enabled = true;
        }

        fn clear_reference(&mut self) {
            self.reference_path = None;
            self.enabled = false;
        }
    }

    let mut menu = YuvDiffMenu {
        enabled: false,
        reference_path: None,
        diff_mode: "absolute".to_string(),
    };

    menu.set_reference("/tmp/ref.yuv".to_string());
    assert!(menu.enabled);
    menu.clear_reference();
    assert!(!menu.enabled);
}

#[test]
fn test_options_menu() {
    struct OptionsMenu {
        show_grid: bool,
        show_motion_vectors: bool,
        show_qp_heatmap: bool,
        auto_save_layout: bool,
    }

    impl OptionsMenu {
        fn toggle_overlay(&mut self, overlay: &str) {
            match overlay {
                "grid" => self.show_grid = !self.show_grid,
                "mv" => self.show_motion_vectors = !self.show_motion_vectors,
                "qp" => self.show_qp_heatmap = !self.show_qp_heatmap,
                _ => {}
            }
        }
    }

    let mut menu = OptionsMenu {
        show_grid: false,
        show_motion_vectors: false,
        show_qp_heatmap: false,
        auto_save_layout: true,
    };

    menu.toggle_overlay("grid");
    assert!(menu.show_grid);
    menu.toggle_overlay("grid");
    assert!(!menu.show_grid);
}

#[test]
fn test_help_menu() {
    struct HelpMenu {
        version: String,
        show_about: bool,
        show_shortcuts: bool,
    }

    impl HelpMenu {
        fn open_about(&mut self) {
            self.show_about = true;
        }

        fn open_shortcuts(&mut self) {
            self.show_shortcuts = true;
        }
    }

    let mut menu = HelpMenu {
        version: "0.1.0".to_string(),
        show_about: false,
        show_shortcuts: false,
    };

    menu.open_about();
    assert!(menu.show_about);
}

#[test]
fn test_submenu() {
    struct Submenu {
        parent: String,
        items: Vec<String>,
        expanded: bool,
    }

    impl Submenu {
        fn toggle(&mut self) {
            self.expanded = !self.expanded;
        }
    }

    let mut submenu = Submenu {
        parent: "Export".to_string(),
        items: vec!["CSV".to_string(), "JSON".to_string()],
        expanded: false,
    };

    submenu.toggle();
    assert!(submenu.expanded);
}

#[test]
fn test_menu_separator() {
    struct MenuSection {
        items: Vec<MenuItem>,
    }

    struct MenuItem {
        label: String,
        is_separator: bool,
    }

    impl MenuSection {
        fn add_separator(&mut self) {
            self.items.push(MenuItem {
                label: String::new(),
                is_separator: true,
            });
        }
    }

    let mut section = MenuSection { items: vec![] };
    section.add_separator();
    assert!(section.items[0].is_separator);
}

#[test]
fn test_checkbox_menu_item() {
    struct CheckboxMenuItem {
        label: String,
        checked: bool,
    }

    impl CheckboxMenuItem {
        fn toggle(&mut self) {
            self.checked = !self.checked;
        }
    }

    let mut item = CheckboxMenuItem {
        label: "Show Status Bar".to_string(),
        checked: true,
    };

    item.toggle();
    assert!(!item.checked);
}

#[test]
fn test_radio_menu_group() {
    struct RadioMenuGroup {
        items: Vec<String>,
        selected_index: usize,
    }

    impl RadioMenuGroup {
        fn select(&mut self, index: usize) {
            if index < self.items.len() {
                self.selected_index = index;
            }
        }

        fn selected(&self) -> Option<&String> {
            self.items.get(self.selected_index)
        }
    }

    let mut group = RadioMenuGroup {
        items: vec!["Small".to_string(), "Medium".to_string(), "Large".to_string()],
        selected_index: 1,
    };

    assert_eq!(group.selected(), Some(&"Medium".to_string()));
    group.select(2);
    assert_eq!(group.selected(), Some(&"Large".to_string()));
}
