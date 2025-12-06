//! Tier 2: Parameter AI
//!
//! This is the second layer of the two-tier AI system.
//! It takes tone parameters from Tier 1 and intelligently maps them to
//! REAPER plugins with precision, using AI to handle the complex mapping.

use crate::ai_client::AIProvider;
use crate::tone_encyclopedia::{EffectParameters, ToneParameters};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

/// REAPER track snapshot (simplified for parameter mapping)
#[derive(Debug, Clone, Serialize)]
pub struct ReaperSnapshot {
    pub track_index: i32,
    pub track_name: String,
    pub plugins: Vec<ReaperPlugin>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReaperPlugin {
    pub index: i32,
    pub name: String,
    pub enabled: bool,
    pub parameters: Vec<ReaperParameter>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReaperParameter {
    pub index: i32,
    pub name: String,
    pub current_value: f64,
    pub display_value: String,
}

/// Mapping action to apply to REAPER
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ParameterAction {
    #[serde(rename = "set_param")]
    SetParameter {
        track: i32,
        plugin_index: i32,
        param_index: i32,
        param_name: String,
        value: f64,
        reason: String,
    },
    #[serde(rename = "enable_plugin")]
    EnablePlugin {
        track: i32,
        plugin_index: i32,
        plugin_name: String,
        reason: String,
    },
    #[serde(rename = "load_plugin")]
    LoadPlugin {
        track: i32,
        plugin_name: String,
        position: Option<i32>,
        reason: String,
    },
}

/// Result from Tier 2 Parameter AI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterAIResult {
    pub actions: Vec<ParameterAction>,
    pub summary: String,
    pub warnings: Vec<String>,
}

/// Tier 2: Parameter AI Engine
pub struct ParameterAI {
    ai_provider: AIProvider,
}

impl ParameterAI {
    /// Create new Parameter AI
    pub fn new(ai_provider: AIProvider) -> Self {
        Self { ai_provider }
    }

    /// Map tone parameters to REAPER actions
    pub async fn map_parameters(
        &self,
        tone_params: &ToneParameters,
        reaper_snapshot: &ReaperSnapshot,
        tone_description: &str,
    ) -> Result<ParameterAIResult, Box<dyn Error>> {
        println!("[PARAMETER AI] Mapping parameters for: {}", tone_description);

        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(tone_params, reaper_snapshot, tone_description);

        let response = self
            .ai_provider
            .generate(&system_prompt, &user_prompt)
            .await?;

        let result = self.parse_ai_response(&response)?;

        println!(
            "[PARAMETER AI] Generated {} actions",
            result.actions.len()
        );

        Ok(result)
    }

    /// Build system prompt for Parameter AI
    fn build_system_prompt(&self) -> String {
        r#"You are a professional REAPER parameter mapping AI specialist.
Your job is to take abstract tone parameters and map them to specific REAPER plugin parameters with precision.

Given:
1. Target tone parameters (amp settings, EQ, effects, etc.)
2. Current REAPER track state (available plugins and their parameters)

Your task:
- Intelligently map each tone parameter to the appropriate REAPER plugin parameter
- Handle parameter name variations (e.g., "gain" might be "Drive", "Gain", "Input", etc.)
- Normalize values correctly (0.0-1.0 for most params, dB for EQ)
- Provide clear reasoning for each mapping
- If a required plugin doesn't exist, suggest loading it

CRITICAL RULES:
1. Only map to parameters that actually exist in the snapshot
2. Use fuzzy matching for parameter names (e.g., "treble" matches "High", "Treble", "HF", etc.)
3. Be conservative with extreme values - avoid clipping
4. Enable plugins before setting their parameters
5. Preserve existing parameter values that aren't being changed

Response format (JSON):
{
  "summary": "Brief description of what you're doing",
  "actions": [
    {
      "type": "set_param",
      "track": 0,
      "plugin_index": 0,
      "param_index": 5,
      "param_name": "Gain",
      "value": 0.85,
      "reason": "Setting amp gain to 0.85 for high-gain tone"
    },
    {
      "type": "enable_plugin",
      "track": 0,
      "plugin_index": 1,
      "plugin_name": "ReaEQ",
      "reason": "Enabling EQ for tone shaping"
    }
  ],
  "warnings": [
    "High gain value may cause clipping - monitor levels"
  ]
}

RESPOND ONLY WITH VALID JSON."#.to_string()
    }

    /// Build user prompt with tone parameters and REAPER state
    fn build_user_prompt(
        &self,
        tone_params: &ToneParameters,
        reaper_snapshot: &ReaperSnapshot,
        tone_description: &str,
    ) -> String {
        let mut prompt = String::new();

        prompt.push_str(&format!("=== TARGET TONE ===\n"));
        prompt.push_str(&format!("Description: {}\n\n", tone_description));

        prompt.push_str("=== TONE PARAMETERS ===\n");

        // Amp parameters
        if !tone_params.amp.is_empty() {
            prompt.push_str("Amp Settings:\n");
            for (key, value) in &tone_params.amp {
                prompt.push_str(&format!("  - {}: {:.3}\n", key, value));
            }
            prompt.push('\n');
        }

        // EQ parameters
        if !tone_params.eq.is_empty() {
            prompt.push_str("EQ Settings (dB):\n");
            for (freq, db) in &tone_params.eq {
                prompt.push_str(&format!("  - {}: {:+.1} dB\n", freq, db));
            }
            prompt.push('\n');
        }

        // Effects
        if !tone_params.effects.is_empty() {
            prompt.push_str("Effects Chain:\n");
            for (i, effect) in tone_params.effects.iter().enumerate() {
                prompt.push_str(&format!("  {}. {} ({})\n", i + 1, effect.effect_type, effect.effect_type));
                for (key, value) in &effect.parameters {
                    prompt.push_str(&format!("     - {}: {:.3}\n", key, value));
                }
            }
            prompt.push('\n');
        }

        // Reverb
        if !tone_params.reverb.is_empty() {
            prompt.push_str("Reverb Settings:\n");
            for (key, value) in &tone_params.reverb {
                prompt.push_str(&format!("  - {}: {:.3}\n", key, value));
            }
            prompt.push('\n');
        }

        // Delay
        if !tone_params.delay.is_empty() {
            prompt.push_str("Delay Settings:\n");
            for (key, value) in &tone_params.delay {
                prompt.push_str(&format!("  - {}: {:.3}\n", key, value));
            }
            prompt.push('\n');
        }

        prompt.push_str("\n=== CURRENT REAPER STATE ===\n");
        prompt.push_str(&format!("Track: {} (index {})\n\n", reaper_snapshot.track_name, reaper_snapshot.track_index));

        prompt.push_str("Available Plugins:\n");
        for plugin in &reaper_snapshot.plugins {
            prompt.push_str(&format!(
                "  Plugin #{}: {} ({})\n",
                plugin.index,
                plugin.name,
                if plugin.enabled { "ENABLED" } else { "DISABLED" }
            ));

            prompt.push_str("    Parameters:\n");
            for param in &plugin.parameters {
                prompt.push_str(&format!(
                    "      [{}] {} = {:.3} ({})\n",
                    param.index, param.name, param.current_value, param.display_value
                ));
            }
            prompt.push('\n');
        }

        prompt.push_str("\n=== YOUR TASK ===\n");
        prompt.push_str("Map the tone parameters to the available REAPER plugins.\n");
        prompt.push_str("Generate actions to achieve the target tone precisely.\n");

        prompt
    }

    /// Parse AI response JSON
    fn parse_ai_response(&self, response: &str) -> Result<ParameterAIResult, Box<dyn Error>> {
        // Extract JSON from markdown code block if present
        let json_str = if response.contains("```json") {
            let start = response.find("```json").unwrap() + 7;
            let end = response[start..].find("```").unwrap() + start;
            &response[start..end]
        } else if response.contains("```") {
            let start = response.find("```").unwrap() + 3;
            let end = response[start..].find("```").unwrap() + start;
            &response[start..end]
        } else if let Some(start) = response.find('{') {
            if let Some(end) = response.rfind('}') {
                &response[start..=end]
            } else {
                response
            }
        } else {
            response
        };

        let parsed: ParameterAIResult = serde_json::from_str(json_str.trim())
            .map_err(|e| format!("Failed to parse Parameter AI response: {}\n\nResponse was:\n{}", e, json_str))?;

        Ok(parsed)
    }

    /// Validate actions before execution
    pub fn validate_actions(
        &self,
        actions: &[ParameterAction],
        reaper_snapshot: &ReaperSnapshot,
    ) -> Vec<String> {
        let mut warnings = Vec::new();

        for action in actions {
            match action {
                ParameterAction::SetParameter {
                    track,
                    plugin_index,
                    param_index,
                    value,
                    ..
                } => {
                    // Check if plugin exists
                    if let Some(plugin) = reaper_snapshot
                        .plugins
                        .iter()
                        .find(|p| p.index == *plugin_index)
                    {
                        // Check if parameter exists
                        if !plugin.parameters.iter().any(|p| p.index == *param_index) {
                            warnings.push(format!(
                                "Parameter {} not found in plugin '{}' (index {})",
                                param_index, plugin.name, plugin_index
                            ));
                        }

                        // Check if plugin is enabled
                        if !plugin.enabled {
                            warnings.push(format!(
                                "Plugin '{}' is disabled - should enable before setting parameters",
                                plugin.name
                            ));
                        }

                        // Check value range
                        if *value < 0.0 || *value > 1.0 {
                            warnings.push(format!(
                                "Parameter value {} is out of range [0.0, 1.0]",
                                value
                            ));
                        }
                    } else {
                        warnings.push(format!(
                            "Plugin index {} not found in track {}",
                            plugin_index, track
                        ));
                    }
                }
                ParameterAction::EnablePlugin { plugin_index, .. } => {
                    if !reaper_snapshot
                        .plugins
                        .iter()
                        .any(|p| p.index == *plugin_index)
                    {
                        warnings.push(format!("Plugin index {} not found", plugin_index));
                    }
                }
                _ => {}
            }
        }

        warnings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parameter_ai_creation() {
        let provider = AIProvider::grok("test-key".to_string(), "grok-beta".to_string());
        let param_ai = ParameterAI::new(provider);

        // Just test instantiation
        assert_eq!(param_ai.ai_provider.name(), "Grok");
    }

    #[test]
    fn test_action_validation() {
        let provider = AIProvider::grok("test-key".to_string(), "grok-beta".to_string());
        let param_ai = ParameterAI::new(provider);

        let snapshot = ReaperSnapshot {
            track_index: 0,
            track_name: "Guitar".to_string(),
            plugins: vec![ReaperPlugin {
                index: 0,
                name: "Amp Simulator".to_string(),
                enabled: true,
                parameters: vec![
                    ReaperParameter {
                        index: 0,
                        name: "Gain".to_string(),
                        current_value: 0.5,
                        display_value: "50%".to_string(),
                    },
                ],
            }],
        };

        // Valid action
        let actions = vec![ParameterAction::SetParameter {
            track: 0,
            plugin_index: 0,
            param_index: 0,
            param_name: "Gain".to_string(),
            value: 0.8,
            reason: "Test".to_string(),
        }];

        let warnings = param_ai.validate_actions(&actions, &snapshot);
        assert!(warnings.is_empty());

        // Invalid action (wrong param index)
        let actions = vec![ParameterAction::SetParameter {
            track: 0,
            plugin_index: 0,
            param_index: 99,
            param_name: "Invalid".to_string(),
            value: 0.8,
            reason: "Test".to_string(),
        }];

        let warnings = param_ai.validate_actions(&actions, &snapshot);
        assert!(!warnings.is_empty());
    }
}
