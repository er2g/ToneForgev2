//! Planner Mode - Analysis and Suggestions
//!
//! This mode analyzes the current REAPER state and provides suggestions.
//! Users can:
//! - Ask for tone analysis
//! - Get improvement suggestions
//! - Discuss potential changes
//! - Plan tone modifications
//!
//! READ-ONLY REAPER access - NO modifications!

use crate::ai_client::AIProvider;
use crate::conversation::{Message, MessageMetadata, MessageRole};
use crate::reaper_client::ReaperClient;
use serde::{Deserialize, Serialize};

const CONTEXT_MESSAGE_LIMIT: usize = 8;

/// Planner mode handler
pub struct PlannerMode {
    reaper_client: ReaperClient,
    ai_provider: AIProvider,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlannerResponse {
    pub content: String,
    pub suggestions: Vec<Suggestion>,
    pub current_state_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub category: SuggestionCategory,
    pub description: String,
    pub priority: Priority,
    pub reasoning: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SuggestionCategory {
    EQ,
    Gain,
    Effects,
    Routing,
    General,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    Medium,
    Low,
}

impl PlannerMode {
    /// Create new planner mode handler
    pub fn new(reaper_client: ReaperClient, ai_provider: AIProvider) -> Self {
        Self {
            reaper_client,
            ai_provider,
        }
    }

    /// Process a planning request
    pub async fn process_message(
        &self,
        user_message: &str,
        conversation_history: &[&Message],
        track_index: i32,
    ) -> Result<PlannerResponse, String> {
        println!("[PLANNER MODE] Processing: {}", user_message);

        // Step 1: Get current REAPER state
        let reaper_state = self
            .collect_reaper_state(track_index)
            .await
            .map_err(|e| format!("Failed to get REAPER state: {}", e))?;

        println!("[PLANNER MODE] Analyzing track: {}", reaper_state.track_name);

        // Step 2: Build AI prompt
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(user_message, conversation_history, &reaper_state);

        // Step 3: Get AI response
        let ai_response = self
            .ai_provider
            .generate(&system_prompt, &user_prompt)
            .await
            .map_err(|e| format!("AI error: {}", e))?;

        // Step 4: Parse suggestions (if structured)
        let suggestions = self.extract_suggestions(&ai_response);

        Ok(PlannerResponse {
            content: ai_response,
            suggestions,
            current_state_summary: reaper_state.summary.clone(),
        })
    }

    async fn collect_reaper_state(&self, track_index: i32) -> Result<ReaperState, Box<dyn std::error::Error>> {
        let overview = self.reaper_client.get_tracks().await?;

        let track = overview
            .tracks
            .iter()
            .find(|t| t.index == track_index)
            .ok_or_else(|| format!("Track {} not found", track_index))?;

        let mut plugins = Vec::new();
        let mut summary_parts = Vec::new();

        for fx in &track.fx_list {
            let params = self.reaper_client.get_fx_params(track_index, fx.index).await?;

            let plugin_summary = PluginSummary {
                index: fx.index,
                name: fx.name.clone(),
                enabled: fx.enabled,
                parameter_count: params.params.len(),
                key_parameters: params
                    .params
                    .iter()
                    .take(5) // First 5 params as preview
                    .map(|p| format!("{}: {}", p.name, p.display))
                    .collect(),
            };

            plugins.push(plugin_summary);

            summary_parts.push(format!(
                "  - {} ({}, {} params)",
                fx.name,
                if fx.enabled { "enabled" } else { "disabled" },
                params.params.len()
            ));
        }

        let summary = format!(
            "Track: {} (index {})\nPlugins: {}\n{}",
            track.name,
            track.index,
            plugins.len(),
            summary_parts.join("\n")
        );

        Ok(ReaperState {
            track_index,
            track_name: track.name.clone(),
            plugins,
            summary,
        })
    }

    fn build_system_prompt(&self) -> String {
        r#"You are a professional audio engineer and tone consultant.

Your role is to analyze REAPER track states and provide expert suggestions.

CAPABILITIES:
- Analyze current plugin chains
- Identify potential issues (muddy EQ, harsh frequencies, etc.)
- Suggest improvements
- Explain tone shaping techniques
- Recommend plugin order and settings

LIMITATIONS:
- You CANNOT modify REAPER (you're in Planner mode - read-only)
- You can only analyze and suggest

ANALYSIS FOCUS:
1. **EQ Issues**: Check for frequency buildup or harsh frequencies
2. **Gain Staging**: Ensure proper levels throughout the chain
3. **Effects Balance**: Verify reverb/delay mix levels
4. **Plugin Order**: Suggest optimal signal flow
5. **Tone Character**: Identify missing elements or over-processing

RESPONSE FORMAT:
- Start with a brief analysis of the current state
- List specific suggestions with reasoning
- Prioritize suggestions (critical, recommended, optional)
- Be constructive and educational

IMPORTANT: Do NOT provide specific parameter values to set.
This mode is for planning and discussion only.
If the user wants to apply changes, suggest they use "Act" mode."#.to_string()
    }

    fn build_user_prompt(
        &self,
        user_message: &str,
        conversation_history: &[&Message],
        reaper_state: &ReaperState,
    ) -> String {
        let mut prompt = String::new();

        // Add conversation context
        if !conversation_history.is_empty() {
            prompt.push_str("=== CONVERSATION HISTORY ===\n");
            for msg in conversation_history.iter().rev().take(CONTEXT_MESSAGE_LIMIT).rev() {
                let role = match msg.role {
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::System => "System",
                };
                prompt.push_str(&format!("{}: {}\n", role, msg.content));
            }
            prompt.push_str("\n");
        }

        // Add REAPER state
        prompt.push_str("=== CURRENT REAPER STATE ===\n");
        prompt.push_str(&format!("Track: {} (index {})\n\n", reaper_state.track_name, reaper_state.track_index));

        if reaper_state.plugins.is_empty() {
            prompt.push_str("No plugins currently loaded on this track.\n");
        } else {
            prompt.push_str("Plugin Chain:\n");
            for plugin in &reaper_state.plugins {
                prompt.push_str(&format!(
                    "{}. {} ({})\n",
                    plugin.index + 1,
                    plugin.name,
                    if plugin.enabled { "ON" } else { "OFF" }
                ));

                if !plugin.key_parameters.is_empty() {
                    prompt.push_str("   Key Parameters:\n");
                    for param in &plugin.key_parameters {
                        prompt.push_str(&format!("   - {}\n", param));
                    }
                }
            }
        }

        prompt.push_str("\n=== USER REQUEST ===\n");
        prompt.push_str(user_message);
        prompt.push('\n');

        prompt
    }

    fn extract_suggestions(&self, ai_response: &str) -> Vec<Suggestion> {
        let mut suggestions = Vec::new();

        // Simple heuristic extraction
        // Look for bullet points or numbered lists
        for line in ai_response.lines() {
            let trimmed = line.trim();

            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }

            // Check for bullet points or numbers
            let is_suggestion = trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || (trimmed.len() > 3
                    && trimmed.chars().nth(0).unwrap_or(' ').is_numeric()
                    && trimmed.contains(". "));

            if is_suggestion {
                let content = if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                    &trimmed[2..]
                } else {
                    trimmed.split(". ").nth(1).unwrap_or(trimmed)
                };

                // Categorize suggestion
                let category = if content.to_lowercase().contains("eq")
                    || content.to_lowercase().contains("frequency")
                    || content.to_lowercase().contains("bass")
                    || content.to_lowercase().contains("treble")
                {
                    SuggestionCategory::EQ
                } else if content.to_lowercase().contains("gain")
                    || content.to_lowercase().contains("drive")
                    || content.to_lowercase().contains("level")
                {
                    SuggestionCategory::Gain
                } else if content.to_lowercase().contains("reverb")
                    || content.to_lowercase().contains("delay")
                    || content.to_lowercase().contains("effect")
                {
                    SuggestionCategory::Effects
                } else if content.to_lowercase().contains("order")
                    || content.to_lowercase().contains("routing")
                    || content.to_lowercase().contains("chain")
                {
                    SuggestionCategory::Routing
                } else {
                    SuggestionCategory::General
                };

                // Determine priority (simple heuristic)
                let priority = if content.to_lowercase().contains("critical")
                    || content.to_lowercase().contains("important")
                    || content.to_lowercase().contains("must")
                {
                    Priority::High
                } else if content.to_lowercase().contains("recommended")
                    || content.to_lowercase().contains("should")
                {
                    Priority::Medium
                } else {
                    Priority::Low
                };

                suggestions.push(Suggestion {
                    category,
                    description: content.to_string(),
                    priority,
                    reasoning: "See detailed explanation above".to_string(),
                });
            }
        }

        suggestions
    }
}

#[derive(Debug, Clone)]
struct ReaperState {
    track_index: i32,
    track_name: String,
    plugins: Vec<PluginSummary>,
    summary: String,
}

#[derive(Debug, Clone)]
struct PluginSummary {
    index: i32,
    name: String,
    enabled: bool,
    parameter_count: usize,
    key_parameters: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planner_mode_creation() {
        let reaper = ReaperClient::new();
        let provider = crate::ai_client::AIProvider::grok("test".to_string(), "test".to_string());
        let _planner = PlannerMode::new(reaper, provider);
    }

    #[test]
    fn test_suggestion_categorization() {
        let reaper = ReaperClient::new();
        let provider = crate::ai_client::AIProvider::grok("test".to_string(), "test".to_string());
        let planner = PlannerMode::new(reaper, provider);

        let response = "Here are my suggestions:\n- Reduce the bass EQ around 200Hz\n- Increase gain to add more saturation";
        let suggestions = planner.extract_suggestions(response);

        assert!(suggestions.len() >= 2);
        assert!(matches!(suggestions[0].category, SuggestionCategory::EQ));
    }
}
