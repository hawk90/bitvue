//! Keyboard and input handling methods for BitvueApp

use crate::bitvue_app::BitvueApp;
use crate::helpers::{count_frames, find_frame_by_index, get_current_frame_index};
use bitvue_core::{Command, StreamId};
use eframe::egui;

/// Input handling methods
pub trait BitvueAppInput {
    fn handle_file_shortcuts(&mut self, ctx: &egui::Context) -> FileShortcutAction;
    fn handle_mode_shortcuts(&mut self, ctx: &egui::Context);
    fn handle_keyboard_navigation(&mut self, ctx: &egui::Context) -> Option<(StreamId, usize)>;
}

/// Actions from file shortcuts (Ctrl+O, Ctrl+W, Ctrl+Q)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileShortcutAction {
    None,
    Open,
    Close,
    Quit,
}

impl BitvueAppInput for BitvueApp {
    /// Handle file shortcuts (Ctrl+O, Ctrl+W, Ctrl+Q)
    fn handle_file_shortcuts(&mut self, ctx: &egui::Context) -> FileShortcutAction {
        // Don't handle keyboard if any widget wants text input
        if ctx.wants_keyboard_input() {
            return FileShortcutAction::None;
        }

        ctx.input(|i| {
            // Ctrl+O - Open file
            if i.key_pressed(egui::Key::O) && i.modifiers.ctrl {
                return FileShortcutAction::Open;
            }
            // Ctrl+W - Close file
            if i.key_pressed(egui::Key::W) && i.modifiers.ctrl {
                return FileShortcutAction::Close;
            }
            // Ctrl+Q - Quit application
            if i.key_pressed(egui::Key::Q) && i.modifiers.ctrl {
                return FileShortcutAction::Quit;
            }
            FileShortcutAction::None
        })
    }

    /// Handle F1-F5 shortcuts for mode switching (VQAnalyzer parity Phase 4)
    fn handle_mode_shortcuts(&mut self, ctx: &egui::Context) {
        // Don't handle keyboard if any widget wants text input
        if ctx.wants_keyboard_input() {
            return;
        }

        let mode_index = ctx.input(|i| {
            if i.key_pressed(egui::Key::F1) {
                Some(0)
            } else if i.key_pressed(egui::Key::F2) {
                Some(1)
            } else if i.key_pressed(egui::Key::F3) {
                Some(2)
            } else if i.key_pressed(egui::Key::F4) {
                Some(3)
            } else if i.key_pressed(egui::Key::F5) {
                Some(4)
            } else {
                None
            }
        });

        if let Some(index) = mode_index {
            // Set mode on all codec workspaces - only the active one matters visually
            // Use accessor methods for lazy workspaces
            self.workspaces.hevc_mut().set_mode_by_index(index);
            self.workspaces.av1_mut().set_mode_by_index(index);
            self.workspaces.vvc_mut().set_mode_by_index(index);
            self.workspaces.avc_mut().set_mode_by_index(index);
            self.workspaces.mpeg2_mut().set_mode_by_index(index);

            tracing::info!("Mode switched to index {} via F{} key", index, index + 1);
        }
    }

    /// Handle keyboard shortcuts for frame navigation
    /// Returns Some((stream_id, frame_index)) if a navigation occurred (triggers decode)
    fn handle_keyboard_navigation(&mut self, ctx: &egui::Context) -> Option<(StreamId, usize)> {
        // Don't handle keyboard if any widget wants text input
        if ctx.wants_keyboard_input() {
            return None;
        }

        let input = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::Home),
                i.key_pressed(egui::Key::End),
            )
        });

        let (left, right, home, end) = input;
        if !left && !right && !home && !end {
            return None;
        }

        // Get current state
        let stream_a = self.core.get_stream(StreamId::A);
        let state = stream_a.read();
        let units = state.units.as_ref()?;
        let total_frames = count_frames(&units.units);
        if total_frames == 0 {
            return None;
        }

        let selection = self.core.get_selection();
        let sel_guard = selection.read();
        let current_index = get_current_frame_index(&sel_guard, &units.units).unwrap_or(0);

        // Calculate new frame index
        let new_index = if left {
            current_index.saturating_sub(1)
        } else if right {
            (current_index + 1).min(total_frames - 1)
        } else if home {
            0
        } else if end {
            total_frames - 1
        } else {
            return None;
        };

        // Skip if same frame
        if new_index == current_index {
            return None;
        }

        // Find the target frame unit
        let target_unit = find_frame_by_index(&units.units, new_index)?;
        let unit_key = target_unit.key.clone();

        // Drop locks before sending command
        drop(sel_guard);
        drop(state);

        // Navigate to frame
        let _events = self.core.handle_command(Command::SelectUnit {
            stream: StreamId::A,
            unit_key,
        });

        Some((StreamId::A, new_index))
    }
}
