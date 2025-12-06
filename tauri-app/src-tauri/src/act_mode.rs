//! Act Mode - Direct Application (Two-Tier System)
//!
//! This mode applies tones directly to REAPER using the two-tier AI system:
//! - Tier 1 (Tone AI): Search encyclopedia or generate tone
//! - Tier 2 (Parameter AI): Map parameters to REAPER and apply
//!
//! FULL REAPER access - applies changes!

use crate::ai_client::AIProvider;
use crate::conversation::{Message, MessageMetadata, MessageRole};
use crate::parameter_ai::{ParameterAction, ParameterAI, ReaperParameter, ReaperPlugin, ReaperSnapshot};
use crate::reaper_client::ReaperClient;
use crate::tone_ai::{ToneAI, ToneSource};
use crate::tone_encyclopedia::ToneEncyclopedia;
use crate::undo_redo::UndoManager;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Act mode handler
pub struct ActMode {
    encyclopedia: ToneEncyclopedia,
    reaper_client: ReaperClient,
    ai_provider: AIProvider,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ActResponse {
    pub tone_source: String,
    pub tone_description: String,
    pub confidence: f32,
    pub summary: String,
    pub actions_count: usize,
    pub action_logs: Vec<String>,
    pub warnings: Vec<String>,
}

impl ActMode {
    /// Create new act mode handler
    pub fn new(
        encyclopedia: ToneEncyclopedia,
        reaper_client: ReaperClient,
        ai_provider: AIProvider,
    ) -> Self {
        Self {
            encyclopedia,
            reaper_client,
            ai_provider,
        }
    }

    /// Process an action request (apply tone to REAPER)
    pub async fn process_message(
        &self,
        user_message: &str,
        track_index: i32,
        undo_manager: &mut UndoManager,
    ) -> Result<ActResponse, String> {
        println!("\n========== ACT MODE: TWO-TIER AI PIPELINE ==========");
        println!("[USER] {}", user_message);

        // ========== TIER 1: TONE AI ==========
        println!("\n[TIER 1] Running Tone AI...");

        let tone_ai = ToneAI::new(self.encyclopedia.clone())
            .with_ai_provider(self.ai_provider.clone());

        let tone_result = tone_ai
            .process_request(user_message)
            .await
            .map_err(|e| format!("Tone AI error: {}", e))?;

        println!("[TIER 1] Result:");
        println!("  - Source: {:?}", tone_result.source);
        println!("  - Description: {}", tone_result.tone_description);
        println!("  - Confidence: {:.0}%", tone_result.confidence * 100.0);

        // ========== GET REAPER SNAPSHOT ==========
        println!("\n[REAPER] Fetching current state...");

        let reaper_snapshot = self
            .collect_reaper_snapshot(track_index)
            .await
            .map_err(|e| format!("Failed to get REAPER state: {}", e))?;

        println!("[REAPER] Track: {}", reaper_snapshot.track_name);
        println!("[REAPER] Plugins: {}", reaper_snapshot.plugins.len());

        // ========== TIER 2: PARAMETER AI ==========
        println!("\n[TIER 2] Running Parameter AI...");

        let parameter_ai = ParameterAI::new(self.ai_provider.clone());

        let parameter_result = parameter_ai
            .map_parameters(
                &tone_result.parameters,
                &reaper_snapshot,
                &tone_result.tone_description,
            )
            .await
            .map_err(|e| format!("Parameter AI error: {}", e))?;

        println!("[TIER 2] Generated {} actions", parameter_result.actions.len());
        println!("[TIER 2] Summary: {}", parameter_result.summary);

        // ========== VALIDATE ACTIONS ==========
        let validation_warnings = parameter_ai.validate_actions(&parameter_result.actions, &reaper_snapshot);

        let mut all_warnings = parameter_result.warnings.clone();
        all_warnings.extend(validation_warnings);

        if !all_warnings.is_empty() {
            println!("\n[VALIDATION] Warnings:");
            for warning in &all_warnings {
                println!("  ⚠️  {}", warning);
            }
        }

        // ========== RECORD FOR UNDO ==========
        undo_manager.begin_action(&format!("Tone: {}", user_message));

        // ========== APPLY ACTIONS TO REAPER ==========
        println!("\n[APPLY] Applying actions to REAPER...");

        let action_logs = self
            .apply_parameter_actions(&parameter_result.actions, &reaper_snapshot, undo_manager)
            .await
            .map_err(|e| format!("Failed to apply actions: {}", e))?;

        for log in &action_logs {
            println!("[ACTION] {}", log);
        }

        // ========== COMMIT UNDO ==========
        if let Some(action_id) = undo_manager.commit_action() {
            println!("[UNDO] Recorded action: {}", action_id);
        }

        println!("\n========== ACT MODE: PIPELINE COMPLETE ==========\n");

        Ok(ActResponse {
            tone_source: format!("{:?}", tone_result.source),
            tone_description: tone_result.tone_description,
            confidence: tone_result.confidence,
            summary: parameter_result.summary,
            actions_count: parameter_result.actions.len(),
            action_logs,
            warnings: all_warnings,
        })
    }

    async fn collect_reaper_snapshot(
        &self,
        track_idx: i32,
    ) -> Result<ReaperSnapshot, Box<dyn Error>> {
        let overview = self.reaper_client.get_tracks().await?;

        let track = overview
            .tracks
            .iter()
            .find(|t| t.index == track_idx)
            .ok_or_else(|| format!("Track {} not found", track_idx))?;

        let mut plugins = Vec::new();

        for fx in &track.fx_list {
            let params_snapshot = self.reaper_client.get_fx_params(track_idx, fx.index).await?;

            let parameters: Vec<ReaperParameter> = params_snapshot
                .params
                .into_iter()
                .map(|p| ReaperParameter {
                    index: p.index,
                    name: p.name,
                    current_value: p.value,
                    display_value: p.display,
                })
                .collect();

            plugins.push(ReaperPlugin {
                index: fx.index,
                name: fx.name.clone(),
                enabled: fx.enabled,
                parameters,
            });
        }

        Ok(ReaperSnapshot {
            track_index: track_idx,
            track_name: track.name.clone(),
            plugins,
        })
    }

    async fn apply_parameter_actions(
        &self,
        actions: &[ParameterAction],
        snapshot: &ReaperSnapshot,
        undo_manager: &mut UndoManager,
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let mut logs = Vec::new();

        for action in actions {
            match action {
                ParameterAction::SetParameter {
                    track,
                    plugin_index,
                    param_index,
                    param_name,
                    value,
                    reason,
                } => {
                    if let Some(plugin) = snapshot.plugins.iter().find(|p| p.index == *plugin_index) {
                        if let Some(param) = plugin.parameters.iter().find(|p| p.index == *param_index) {
                            // Record for undo
                            undo_manager.record_param_change(
                                *track,
                                *plugin_index,
                                *param_index,
                                param_name,
                                param.current_value,
                                *value,
                            );

                            // Apply change
                            self.reaper_client.set_param(*track, *plugin_index, param_name, *value).await?;

                            logs.push(format!(
                                "✓ {} :: {} = {:.1}% (was {:.1}%) - {}",
                                plugin.name,
                                param_name,
                                value * 100.0,
                                param.current_value * 100.0,
                                reason
                            ));
                        }
                    }
                }
                ParameterAction::EnablePlugin {
                    track,
                    plugin_index,
                    plugin_name,
                    reason,
                } => {
                    if let Some(plugin) = snapshot.plugins.iter().find(|p| p.index == *plugin_index) {
                        undo_manager.record_fx_toggle(*track, *plugin_index, plugin_name, plugin.enabled);
                    }

                    self.reaper_client.set_fx_enabled(*track, *plugin_index, true).await?;

                    logs.push(format!("✓ Enabled '{}' - {}", plugin_name, reason));
                }
                ParameterAction::LoadPlugin {
                    track,
                    plugin_name,
                    reason,
                    ..
                } => {
                    let slot = self.reaper_client.add_plugin(*track, plugin_name).await?;

                    logs.push(format!(
                        "✓ Loaded '{}' at slot {} - {}",
                        plugin_name, slot, reason
                    ));
                }
            }
        }

        Ok(logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_act_mode_creation() {
        let encyclopedia = ToneEncyclopedia::new();
        let reaper = ReaperClient::new();
        let provider = crate::ai_client::AIProvider::grok("test".to_string(), "test".to_string());

        let _act_mode = ActMode::new(encyclopedia, reaper, provider);
    }
}
