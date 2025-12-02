// AI Engine: Professional-grade algorithms for REAPER control
//
// This module provides:
// 1. State diffing (detect what actually changed)
// 2. Action optimization (merge, deduplicate, reorder)
// 3. Semantic parameter grouping (gain/drive/overdrive → Distortion)
// 4. Safety validation (bounds checking, conflict detection)
// 5. Parameter relationship modeling (gain ↑ → bass ↓)
// 6. Transaction support (rollback on failure)

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// ============================================================================
// SEMANTIC PARAMETER CATEGORIES
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParameterCategory {
    Distortion,   // gain, drive, overdrive, saturation
    EQ,           // bass, mid, treble, low, high
    Dynamics,     // compression, threshold, ratio
    Modulation,   // chorus, flanger, phaser, rate, depth
    Delay,        // delay time, feedback, mix
    Reverb,       // room size, decay, damping
    Filter,       // cutoff, resonance, Q
    Volume,       // level, output, mix
    Toggle,       // on/off switches, bypasses
    Unknown,
}

pub struct SemanticAnalyzer;

impl SemanticAnalyzer {
    pub fn categorize(param_name: &str) -> ParameterCategory {
        let lower = param_name.to_lowercase();

        // Distortion category
        if lower.contains("gain")
            || lower.contains("drive")
            || lower.contains("overdrive")
            || lower.contains("distortion")
            || lower.contains("saturation") {
            return ParameterCategory::Distortion;
        }

        // EQ category
        if lower.contains("bass")
            || lower.contains("mid")
            || lower.contains("treble")
            || lower.contains("low")
            || lower.contains("high")
            || lower.contains("eq") {
            return ParameterCategory::EQ;
        }

        // Dynamics
        if lower.contains("comp")
            || lower.contains("threshold")
            || lower.contains("ratio")
            || lower.contains("attack")
            || lower.contains("release") {
            return ParameterCategory::Dynamics;
        }

        // Modulation
        if lower.contains("chorus")
            || lower.contains("flanger")
            || lower.contains("phaser")
            || lower.contains("rate")
            || lower.contains("depth")
            || lower.contains("modulation") {
            return ParameterCategory::Modulation;
        }

        // Delay
        if lower.contains("delay")
            || lower.contains("echo")
            || lower.contains("feedback") {
            return ParameterCategory::Delay;
        }

        // Reverb
        if lower.contains("reverb")
            || lower.contains("room")
            || lower.contains("decay")
            || lower.contains("damping") {
            return ParameterCategory::Reverb;
        }

        // Filter
        if lower.contains("filter")
            || lower.contains("cutoff")
            || lower.contains("resonance")
            || lower.contains("q") {
            return ParameterCategory::Filter;
        }

        // Volume
        if lower.contains("volume")
            || lower.contains("level")
            || lower.contains("output")
            || lower.contains("mix") {
            return ParameterCategory::Volume;
        }

        // Toggle
        if lower.contains("enable")
            || lower.contains("bypass")
            || lower.contains("on")
            || lower.contains("active") {
            return ParameterCategory::Toggle;
        }

        ParameterCategory::Unknown
    }
}

// ============================================================================
// STATE DIFFING
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ParameterDiff {
    pub track: i32,
    pub fx_index: i32,
    pub param_index: i32,
    pub param_name: String,
    pub old_value: f64,
    pub new_value: f64,
    pub old_display: String,
    pub new_display: String,
    pub delta: f64, // new - old
}

#[derive(Debug, Clone, Serialize)]
pub struct StateDiff {
    pub changed_params: Vec<ParameterDiff>,
    pub new_fx: Vec<String>,
    pub removed_fx: Vec<String>,
    pub toggled_fx: Vec<(String, bool)>, // (name, new_enabled_state)
}

pub struct StateDiffer;

impl StateDiffer {
    pub fn diff(
        old_state: &[(i32, Vec<(i32, String, bool, Vec<(i32, String, f64, String)>)>)],
        new_state: &[(i32, Vec<(i32, String, bool, Vec<(i32, String, f64, String)>)>)],
    ) -> StateDiff {
        let mut changed_params = Vec::new();
        let mut new_fx = Vec::new();
        let mut removed_fx = Vec::new();
        let mut toggled_fx = Vec::new();

        // Build maps for efficient lookup
        let old_map: HashMap<_, _> = old_state.iter().cloned().collect();
        let new_map: HashMap<_, _> = new_state.iter().cloned().collect();

        for (track_idx, new_fx_list) in new_state {
            if let Some(old_fx_list) = old_map.get(track_idx) {
                // Compare FX lists
                let old_fx_map: HashMap<_, _> = old_fx_list
                    .iter()
                    .map(|(idx, name, enabled, params)| (*idx, (name.clone(), *enabled, params.clone())))
                    .collect();
                let new_fx_map: HashMap<_, _> = new_fx_list
                    .iter()
                    .map(|(idx, name, enabled, params)| (*idx, (name.clone(), *enabled, params.clone())))
                    .collect();

                // Detect new FX
                for fx_idx in new_fx_map.keys() {
                    if !old_fx_map.contains_key(fx_idx) {
                        if let Some((name, _, _)) = new_fx_map.get(fx_idx) {
                            new_fx.push(name.clone());
                        }
                    }
                }

                // Detect removed FX
                for fx_idx in old_fx_map.keys() {
                    if !new_fx_map.contains_key(fx_idx) {
                        if let Some((name, _, _)) = old_fx_map.get(fx_idx) {
                            removed_fx.push(name.clone());
                        }
                    }
                }

                // Compare parameters for existing FX
                for (fx_idx, (new_name, new_enabled, new_params)) in &new_fx_map {
                    if let Some((old_name, old_enabled, old_params)) = old_fx_map.get(fx_idx) {
                        // Check if enabled state changed
                        if old_enabled != new_enabled {
                            toggled_fx.push((new_name.clone(), *new_enabled));
                        }

                        // Compare parameters
                        let old_params_map: HashMap<_, _> = old_params
                            .iter()
                            .map(|(idx, name, val, display)| (*idx, (name, *val, display)))
                            .collect();

                        for (param_idx, param_name, new_val, new_display) in new_params {
                            if let Some((_, old_val, old_display)) = old_params_map.get(param_idx) {
                                let delta = new_val - old_val;
                                if delta.abs() > 0.001 {
                                    // Threshold for floating point comparison
                                    changed_params.push(ParameterDiff {
                                        track: *track_idx,
                                        fx_index: *fx_idx,
                                        param_index: *param_idx,
                                        param_name: param_name.clone(),
                                        old_value: *old_val,
                                        new_value: *new_val,
                                        old_display: old_display.to_string(),
                                        new_display: new_display.clone(),
                                        delta,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        StateDiff {
            changed_params,
            new_fx,
            removed_fx,
            toggled_fx,
        }
    }
}

// ============================================================================
// ACTION OPTIMIZER
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub track: i32,
    pub fx_index: i32,
    pub param_index: i32,
    pub value: f64,
    pub reason: String,
}

pub struct ActionOptimizer;

impl ActionOptimizer {
    /// Removes duplicate actions (same param modified multiple times)
    /// Keeps only the LAST modification
    pub fn deduplicate(actions: Vec<ActionPlan>) -> Vec<ActionPlan> {
        let mut map: HashMap<(i32, i32, i32), ActionPlan> = HashMap::new();

        for action in actions {
            let key = (action.track, action.fx_index, action.param_index);
            map.insert(key, action);
        }

        map.into_values().collect()
    }

    /// Detects conflicts (impossible combinations)
    /// Example: Setting gain to 0.9 AND 0.3 in same batch
    pub fn detect_conflicts(actions: &[ActionPlan]) -> Vec<String> {
        let mut conflicts = Vec::new();
        let mut seen: HashMap<(i32, i32, i32), Vec<f64>> = HashMap::new();

        for action in actions {
            let key = (action.track, action.fx_index, action.param_index);
            seen.entry(key).or_default().push(action.value);
        }

        for (key, values) in seen {
            if values.len() > 1 {
                let unique_values: HashSet<_> = values.iter().map(|v| (v * 1000.0) as i64).collect();
                if unique_values.len() > 1 {
                    conflicts.push(format!(
                        "Conflict detected: Track {} FX {} Param {} set to multiple values: {:?}",
                        key.0, key.1, key.2, values
                    ));
                }
            }
        }

        conflicts
    }

    /// Reorders actions for optimal execution
    /// 1. Enable plugins first
    /// 2. Enable sections/pedals
    /// 3. Modify parameters
    pub fn reorder(actions: Vec<ActionPlan>) -> Vec<ActionPlan> {
        let mut toggle_actions = Vec::new();
        let mut param_actions = Vec::new();

        for action in actions {
            // Heuristic: if param name contains "enable" or value is 0/1, it's a toggle
            if action.value == 0.0 || action.value == 1.0 {
                toggle_actions.push(action);
            } else {
                param_actions.push(action);
            }
        }

        // Toggles first, then params
        toggle_actions.extend(param_actions);
        toggle_actions
    }
}

// ============================================================================
// SAFETY VALIDATOR
// ============================================================================

#[derive(Debug, Clone)]
pub struct SafetyBounds {
    pub min: f64,
    pub max: f64,
    pub recommended_max: f64,
}

pub struct SafetyValidator;

impl SafetyValidator {
    /// Get safe bounds for a parameter based on its category
    pub fn get_bounds(category: &ParameterCategory) -> SafetyBounds {
        match category {
            ParameterCategory::Distortion => SafetyBounds {
                min: 0.0,
                max: 1.0,
                recommended_max: 0.9, // Avoid extreme distortion
            },
            ParameterCategory::EQ => SafetyBounds {
                min: 0.0,
                max: 1.0,
                recommended_max: 0.85, // Avoid extreme boosts
            },
            ParameterCategory::Volume => SafetyBounds {
                min: 0.0,
                max: 1.0,
                recommended_max: 0.95, // Avoid clipping
            },
            _ => SafetyBounds {
                min: 0.0,
                max: 1.0,
                recommended_max: 1.0,
            },
        }
    }

    /// Validate and clamp value to safe range
    pub fn validate_value(
        param_name: &str,
        value: f64,
    ) -> (f64, Option<String>) {
        let category = SemanticAnalyzer::categorize(param_name);
        let bounds = Self::get_bounds(&category);

        let mut warnings = Vec::new();
        let mut clamped_value = value;

        // Hard limit
        if value < bounds.min {
            warnings.push(format!("Value {} below minimum {}, clamping", value, bounds.min));
            clamped_value = bounds.min;
        } else if value > bounds.max {
            warnings.push(format!("Value {} above maximum {}, clamping", value, bounds.max));
            clamped_value = bounds.max;
        }

        // Soft warning for extreme values
        if value > bounds.recommended_max {
            warnings.push(format!(
                "⚠️  Value {} exceeds recommended max {} for {:?}. May cause clipping/distortion.",
                value, bounds.recommended_max, category
            ));
        }

        (clamped_value, if warnings.is_empty() { None } else { Some(warnings.join("; ")) })
    }
}

// ============================================================================
// PARAMETER RELATIONSHIP ENGINE
// ============================================================================

pub struct RelationshipEngine;

impl RelationshipEngine {
    /// Suggests compensatory adjustments based on parameter relationships
    /// Example: If gain is increased significantly, suggest reducing bass
    pub fn suggest_compensations(
        param_name: &str,
        old_value: f64,
        new_value: f64,
    ) -> Vec<(String, f64, String)> {
        let mut suggestions = Vec::new();
        let delta = new_value - old_value;
        let category = SemanticAnalyzer::categorize(param_name);

        match category {
            ParameterCategory::Distortion => {
                if delta > 0.2 {
                    // Significant gain increase
                    suggestions.push((
                        "bass".to_string(),
                        -0.1,
                        "High gain can cause muddiness, reduce bass".to_string(),
                    ));
                    suggestions.push((
                        "mid".to_string(),
                        -0.05,
                        "Scoop mids slightly for tighter sound".to_string(),
                    ));
                }
            }
            ParameterCategory::EQ => {
                if param_name.to_lowercase().contains("treble") && delta > 0.2 {
                    suggestions.push((
                        "mid".to_string(),
                        0.1,
                        "Balance treble boost with mid increase".to_string(),
                    ));
                }
            }
            _ => {}
        }

        suggestions
    }
}

// ============================================================================
// TRANSACTION SUPPORT (for rollback)
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct Transaction {
    pub id: String,
    pub actions: Vec<ActionPlan>,
    pub original_state: Vec<ParameterDiff>,
}

impl Transaction {
    pub fn new(actions: Vec<ActionPlan>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            actions,
            original_state: Vec::new(),
        }
    }

    pub fn with_state(mut self, state: Vec<ParameterDiff>) -> Self {
        self.original_state = state;
        self
    }

    /// Generate rollback actions to restore original state
    pub fn rollback_actions(&self) -> Vec<ActionPlan> {
        self.original_state
            .iter()
            .map(|diff| ActionPlan {
                track: diff.track,
                fx_index: diff.fx_index,
                param_index: diff.param_index,
                value: diff.old_value,
                reason: format!("Rollback transaction {}", self.id),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semantic_categorization() {
        assert_eq!(SemanticAnalyzer::categorize("Gain"), ParameterCategory::Distortion);
        assert_eq!(SemanticAnalyzer::categorize("Overdrive"), ParameterCategory::Distortion);
        assert_eq!(SemanticAnalyzer::categorize("Bass"), ParameterCategory::EQ);
        assert_eq!(SemanticAnalyzer::categorize("Treble"), ParameterCategory::EQ);
    }

    #[test]
    fn test_action_deduplication() {
        let actions = vec![
            ActionPlan {
                track: 0,
                fx_index: 0,
                param_index: 1,
                value: 0.5,
                reason: "First".to_string(),
            },
            ActionPlan {
                track: 0,
                fx_index: 0,
                param_index: 1,
                value: 0.8,
                reason: "Second".to_string(),
            },
        ];

        let deduped = ActionOptimizer::deduplicate(actions);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].value, 0.8); // Keeps last one
    }

    #[test]
    fn test_safety_validation() {
        let (clamped, warning) = SafetyValidator::validate_value("Gain", 1.5);
        assert_eq!(clamped, 1.0);
        assert!(warning.is_some());
    }
}
