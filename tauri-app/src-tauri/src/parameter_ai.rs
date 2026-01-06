//! Tier 2: Parameter AI
//!
//! This is the second layer of the two-tier AI system.
//! It takes tone parameters from Tier 1 and intelligently maps them to
//! REAPER plugins with precision, using AI to handle the complex mapping.

use crate::ai_client::AIProvider;
use crate::tone_encyclopedia::ToneParameters;
use serde::{Deserialize, Serialize};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct ParameterAIOptions {
    pub allow_load_plugins: bool,
    pub max_actions: usize,
    pub phase_name: String,
}

impl Default for ParameterAIOptions {
    fn default() -> Self {
        Self {
            allow_load_plugins: true,
            max_actions: 160,
            phase_name: "map".to_string(),
        }
    }
}

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
    pub unit: String,
    pub format_hint: String,
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
        self.map_parameters_with_options(
            tone_params,
            reaper_snapshot,
            tone_description,
            &ParameterAIOptions::default(),
            None,
        )
        .await
    }

    pub async fn map_parameters_with_options(
        &self,
        tone_params: &ToneParameters,
        reaper_snapshot: &ReaperSnapshot,
        tone_description: &str,
        options: &ParameterAIOptions,
        additional_instructions: Option<&str>,
    ) -> Result<ParameterAIResult, Box<dyn Error>> {
        println!(
            "[PARAMETER AI] Mapping parameters for: {} (phase={}, allow_loads={})",
            tone_description, options.phase_name, options.allow_load_plugins
        );

        let system_prompt = self.build_system_prompt(options);
        let user_prompt =
            self.build_user_prompt(tone_params, reaper_snapshot, tone_description, additional_instructions);

        let response = self.ai_provider.generate(&system_prompt, &user_prompt).await?;
        let mut parsed = self.parse_ai_response(&response)?;

        let issues = self.validate_actions_strict(&parsed.actions, reaper_snapshot, options);
        if !issues.is_empty() {
            println!(
                "[PARAMETER AI] Validation issues found; attempting repair: {:?}",
                issues
            );
            let repair_prompt = format!(
                "{user_prompt}\n\nYour previous JSON has validation errors:\n- {}\n\nReturn corrected JSON ONLY.\n\nPrevious output:\n{response}",
                issues.join("\n- ")
            );
            if let Ok(repair_response) = self.ai_provider.generate(&system_prompt, &repair_prompt).await {
                if let Ok(repair_parsed) = self.parse_ai_response(&repair_response) {
                    parsed = repair_parsed;
                }
            }
        }

        if parsed.actions.len() > options.max_actions {
            parsed.warnings.push(format!(
                "Model returned {} actions; capping to {} for safety",
                parsed.actions.len(),
                options.max_actions
            ));
            parsed.actions.truncate(options.max_actions);
        }

        println!("[PARAMETER AI] Generated {} actions", parsed.actions.len());
        Ok(parsed)
    }

    /// Build system prompt for Parameter AI
    fn build_system_prompt(&self, options: &ParameterAIOptions) -> String {
        let load_rule = if options.allow_load_plugins {
            "You MAY include 'load_plugin' actions if needed.\nIMPORTANT: If you include any 'load_plugin' actions, do NOT include 'set_param' actions for plugins that are not already present in the provided snapshot. Newly loaded plugins will be mapped in a later phase."
        } else {
            "You MUST NOT include any 'load_plugin' actions in this phase. Use only plugins already present in the provided snapshot."
        };

        format!(
            r#"You are a senior REAPER automation and mix engineer agent.
Your job is to map abstract tone parameters to concrete REAPER actions.

Core rules:
- Only use plugin_index values that exist in the provided snapshot.
- Only use param_index values that exist under that plugin in the provided snapshot.
- Keep values in [0.0, 1.0] (normalized).
- Enable plugins before setting their parameters.
- Keep the action list under {} actions.

Phase rule:
{}

Output JSON schema:
{{
  "summary": "1-2 sentences",
  "actions": [
    {{"type":"enable_plugin","track":0,"plugin_index":0,"plugin_name":"...","reason":"..."}},
    {{"type":"set_param","track":0,"plugin_index":0,"param_index":1,"param_name":"...","value":0.5,"reason":"..."}},
    {{"type":"load_plugin","track":0,"plugin_name":"...","position":null,"reason":"..."}}
  ],
  "warnings": []
}}

Return ONLY valid JSON. No markdown."#,
            options.max_actions, load_rule
        )
    }

    /// Build user prompt with tone parameters and REAPER state
    fn build_user_prompt(
        &self,
        tone_params: &ToneParameters,
        reaper_snapshot: &ReaperSnapshot,
        tone_description: &str,
        additional_instructions: Option<&str>,
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
        if let Some(extra) = additional_instructions
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            prompt.push_str("\n=== ADDITIONAL INSTRUCTIONS ===\n");
            prompt.push_str(extra);
            prompt.push('\n');
        }

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
        self.validate_actions_strict(actions, reaper_snapshot, &ParameterAIOptions::default())
    }

    fn validate_actions_strict(
        &self,
        actions: &[ParameterAction],
        reaper_snapshot: &ReaperSnapshot,
        options: &ParameterAIOptions,
    ) -> Vec<String> {
        let mut issues = Vec::new();

        if actions.len() > options.max_actions {
            issues.push(format!(
                "Too many actions ({}), must be <= {}",
                actions.len(),
                options.max_actions
            ));
        }

        for action in actions {
            match action {
                ParameterAction::SetParameter {
                    track,
                    plugin_index,
                    param_index,
                    value,
                    ..
                } => {
                    if *track != reaper_snapshot.track_index {
                        issues.push(format!(
                            "SetParameter uses track {} but snapshot track is {}",
                            track, reaper_snapshot.track_index
                        ));
                    }

                    let Some(plugin) = reaper_snapshot.plugins.iter().find(|p| p.index == *plugin_index) else {
                        issues.push(format!(
                            "SetParameter references missing plugin_index {}",
                            plugin_index
                        ));
                        continue;
                    };

                    if !plugin.parameters.iter().any(|p| p.index == *param_index) {
                        issues.push(format!(
                            "SetParameter references missing param_index {} on plugin '{}'(#{})",
                            param_index, plugin.name, plugin_index
                        ));
                    }

                    if !(0.0..=1.0).contains(value) {
                        issues.push(format!(
                            "SetParameter value {} out of range [0.0, 1.0] for plugin '{}'",
                            value, plugin.name
                        ));
                    }
                }
                ParameterAction::EnablePlugin { track, plugin_index, .. } => {
                    if *track != reaper_snapshot.track_index {
                        issues.push(format!(
                            "EnablePlugin uses track {} but snapshot track is {}",
                            track, reaper_snapshot.track_index
                        ));
                    }
                    if !reaper_snapshot.plugins.iter().any(|p| p.index == *plugin_index) {
                        issues.push(format!(
                            "EnablePlugin references missing plugin_index {}",
                            plugin_index
                        ));
                    }
                }
                ParameterAction::LoadPlugin { track, plugin_name, .. } => {
                    if *track != reaper_snapshot.track_index {
                        issues.push(format!(
                            "LoadPlugin uses track {} but snapshot track is {}",
                            track, reaper_snapshot.track_index
                        ));
                    }
                    if !options.allow_load_plugins {
                        issues.push("LoadPlugin is not allowed in this phase".to_string());
                    }
                    if plugin_name.trim().is_empty() {
                        issues.push("LoadPlugin has empty plugin_name".to_string());
                    }
                }
            }
        }

        issues
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
                        unit: "%".to_string(),
                        format_hint: "percentage".to_string(),
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
