//! Undo/Redo System for ToneForge
//!
//! This module provides a transaction-based undo/redo system for plugin parameter changes.
//! Each user action creates a snapshot that can be reverted or re-applied.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MAX_UNDO_HISTORY: usize = 50;

/// Represents a single parameter change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterChange {
    pub track: i32,
    pub fx_index: i32,
    #[serde(default)]
    pub fx_name: String,
    pub param_index: i32,
    pub param_name: String,
    pub old_value: f64,
    pub new_value: f64,
}

/// Represents a single FX toggle change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxToggleChange {
    pub track: i32,
    pub fx_index: i32,
    pub fx_name: String,
    pub was_enabled: bool,
}

/// Represents a plugin load/unload change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginChange {
    pub track: i32,
    pub fx_index: i32,
    pub plugin_name: String,
    pub was_loaded: bool, // true = was loaded (undo = remove), false = was removed (undo = add)
}

/// A single action that can contain multiple changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoAction {
    pub id: String,
    pub description: String,
    pub timestamp: u64,
    pub parameter_changes: Vec<ParameterChange>,
    pub fx_toggles: Vec<FxToggleChange>,
    pub plugin_changes: Vec<PluginChange>,
}

impl UndoAction {
    pub fn new(description: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            description: description.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            parameter_changes: Vec::new(),
            fx_toggles: Vec::new(),
            plugin_changes: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.parameter_changes.is_empty()
            && self.fx_toggles.is_empty()
            && self.plugin_changes.is_empty()
    }

    pub fn add_param_change(&mut self, change: ParameterChange) {
        self.parameter_changes.push(change);
    }

    pub fn add_fx_toggle(&mut self, toggle: FxToggleChange) {
        self.fx_toggles.push(toggle);
    }

    pub fn add_plugin_change(&mut self, change: PluginChange) {
        self.plugin_changes.push(change);
    }

    /// Get total number of changes in this action
    pub fn change_count(&self) -> usize {
        self.parameter_changes.len() + self.fx_toggles.len() + self.plugin_changes.len()
    }
}

/// The main undo/redo manager
#[derive(Debug, Default)]
pub struct UndoManager {
    undo_stack: VecDeque<UndoAction>,
    redo_stack: VecDeque<UndoAction>,
    current_action: Option<UndoAction>,
}

impl UndoManager {
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::with_capacity(MAX_UNDO_HISTORY),
            redo_stack: VecDeque::with_capacity(MAX_UNDO_HISTORY),
            current_action: None,
        }
    }

    /// Start a new action group (for batching multiple changes)
    pub fn begin_action(&mut self, description: &str) {
        self.current_action = Some(UndoAction::new(description));
    }

    /// Record a parameter change in the current action
    pub fn record_param_change(
        &mut self,
        track: i32,
        fx_index: i32,
        fx_name: &str,
        param_index: i32,
        param_name: &str,
        old_value: f64,
        new_value: f64,
    ) {
        if let Some(ref mut action) = self.current_action {
            action.add_param_change(ParameterChange {
                track,
                fx_index,
                fx_name: fx_name.to_string(),
                param_index,
                param_name: param_name.to_string(),
                old_value,
                new_value,
            });
        }
    }

    /// Record an FX toggle in the current action
    pub fn record_fx_toggle(
        &mut self,
        track: i32,
        fx_index: i32,
        fx_name: &str,
        was_enabled: bool,
    ) {
        if let Some(ref mut action) = self.current_action {
            action.add_fx_toggle(FxToggleChange {
                track,
                fx_index,
                fx_name: fx_name.to_string(),
                was_enabled,
            });
        }
    }

    /// Record a plugin load/unload in the current action
    pub fn record_plugin_change(
        &mut self,
        track: i32,
        fx_index: i32,
        plugin_name: &str,
        was_loaded: bool,
    ) {
        if let Some(ref mut action) = self.current_action {
            action.add_plugin_change(PluginChange {
                track,
                fx_index,
                plugin_name: plugin_name.to_string(),
                was_loaded,
            });
        }
    }

    /// Commit the current action to the undo stack
    pub fn commit_action(&mut self) -> Option<String> {
        if let Some(action) = self.current_action.take() {
            if !action.is_empty() {
                let id = action.id.clone();

                // Clear redo stack when new action is committed
                self.redo_stack.clear();

                // Add to undo stack
                self.undo_stack.push_back(action);

                // Trim if too many items
                while self.undo_stack.len() > MAX_UNDO_HISTORY {
                    self.undo_stack.pop_front();
                }

                return Some(id);
            }
        }
        None
    }

    /// Cancel the current action without committing
    pub fn cancel_action(&mut self) {
        self.current_action = None;
    }

    /// Pop the last action from undo stack (for applying undo)
    pub fn pop_undo(&mut self) -> Option<UndoAction> {
        self.undo_stack.pop_back()
    }

    /// Push an action to redo stack (after undo is applied)
    pub fn push_redo(&mut self, action: UndoAction) {
        self.redo_stack.push_back(action);
        while self.redo_stack.len() > MAX_UNDO_HISTORY {
            self.redo_stack.pop_front();
        }
    }

    /// Pop the last action from redo stack (for applying redo)
    pub fn pop_redo(&mut self) -> Option<UndoAction> {
        self.redo_stack.pop_back()
    }

    /// Push an action to undo stack (after redo is applied)
    pub fn push_undo(&mut self, action: UndoAction) {
        self.undo_stack.push_back(action);
        while self.undo_stack.len() > MAX_UNDO_HISTORY {
            self.undo_stack.pop_front();
        }
    }

    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the description of the next undo action
    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.back().map(|a| a.description.as_str())
    }

    /// Get the description of the next redo action
    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.back().map(|a| a.description.as_str())
    }

    pub fn last_undo_action(&self) -> Option<UndoAction> {
        self.undo_stack.back().cloned()
    }

    /// Get undo stack size
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get redo stack size
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Get recent undo history (for UI display)
    pub fn get_undo_history(&self, limit: usize) -> Vec<UndoActionSummary> {
        self.undo_stack
            .iter()
            .rev()
            .take(limit)
            .map(|a| UndoActionSummary {
                id: a.id.clone(),
                description: a.description.clone(),
                change_count: a.change_count(),
                timestamp: a.timestamp,
            })
            .collect()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.current_action = None;
    }
}

/// Summary of an undo action (for UI display)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoActionSummary {
    pub id: String,
    pub description: String,
    pub change_count: usize,
    pub timestamp: u64,
}

/// State returned to frontend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoState {
    pub can_undo: bool,
    pub can_redo: bool,
    pub undo_description: Option<String>,
    pub redo_description: Option<String>,
    pub undo_count: usize,
    pub redo_count: usize,
}

impl From<&UndoManager> for UndoState {
    fn from(manager: &UndoManager) -> Self {
        Self {
            can_undo: manager.can_undo(),
            can_redo: manager.can_redo(),
            undo_description: manager.undo_description().map(String::from),
            redo_description: manager.redo_description().map(String::from),
            undo_count: manager.undo_count(),
            redo_count: manager.redo_count(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_manager_basic() {
        let mut manager = UndoManager::new();

        // Should start empty
        assert!(!manager.can_undo());
        assert!(!manager.can_redo());

        // Begin and commit an action
        manager.begin_action("Test action");
        manager.record_param_change(0, 0, "Amp", 1, "Gain", 0.5, 0.8);
        manager.commit_action();

        // Should now have undo available
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
        assert_eq!(manager.undo_description(), Some("Test action"));
    }

    #[test]
    fn test_undo_redo_cycle() {
        let mut manager = UndoManager::new();

        // Create action
        manager.begin_action("Change gain");
        manager.record_param_change(0, 0, "Amp", 1, "Gain", 0.5, 0.8);
        manager.commit_action();

        // Undo
        let action = manager.pop_undo().unwrap();
        assert_eq!(action.description, "Change gain");
        manager.push_redo(action);

        // Should now have redo available
        assert!(!manager.can_undo());
        assert!(manager.can_redo());

        // Redo
        let action = manager.pop_redo().unwrap();
        manager.push_undo(action);

        // Back to undo available
        assert!(manager.can_undo());
        assert!(!manager.can_redo());
    }

    #[test]
    fn test_empty_action_not_committed() {
        let mut manager = UndoManager::new();

        manager.begin_action("Empty action");
        let result = manager.commit_action();

        assert!(result.is_none());
        assert!(!manager.can_undo());
    }

    #[test]
    fn test_new_action_clears_redo() {
        let mut manager = UndoManager::new();

        // Create and undo an action
        manager.begin_action("Action 1");
        manager.record_param_change(0, 0, "Amp", 1, "Gain", 0.5, 0.8);
        manager.commit_action();

        let action = manager.pop_undo().unwrap();
        manager.push_redo(action);
        assert!(manager.can_redo());

        // New action should clear redo
        manager.begin_action("Action 2");
        manager.record_param_change(0, 0, "Amp", 2, "Bass", 0.3, 0.6);
        manager.commit_action();

        assert!(!manager.can_redo());
    }
}
